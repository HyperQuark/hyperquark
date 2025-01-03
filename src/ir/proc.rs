use super::{Step, StepContext, Target, Type as IrType};
use crate::prelude::*;
use crate::sb3::{BlockMap, BlockOpcode};
use lazy_regex::{lazy_regex, Lazy};
use regex::Regex;

#[derive(Clone, Debug)]
pub struct ProcedureContext {
    arg_ids: Box<[Box<str>]>,
    arg_types: Box<[IrType]>,
    warp: bool,
    target: Rc<Target>,
}

impl ProcedureContext {
    pub fn arg_ids(&self) -> &[Box<str>] {
        self.arg_ids.borrow()
    }

    pub fn arg_types(&self) -> &[IrType] {
        self.arg_types.borrow()
    }

    pub fn warp(&self) -> bool {
        self.warp
    }

    pub fn target_context(&self) -> &Target {
        self.target.borrow()
    }
}

#[derive(Clone)]
pub struct Proc {
    first_step: Box<Step>,
    context: ProcedureContext,
}

impl Proc {
    pub fn first_step(&self) -> &Step {
        self.first_step.borrow()
    }

    pub fn context(&self) -> &ProcedureContext {
        &self.context
    }
}

static ARG_REGEX: Lazy<Regex> = lazy_regex!(r#"[^\\]%[nbs]"#);

fn arg_types_from_proccode(proccode: Box<str>) -> Result<Box<[IrType]>, HQError> {
    // https://github.com/scratchfoundation/scratch-blocks/blob/abbfe93136fef57fdfb9a077198b0bc64726f012/blocks_vertical/procedures.js#L207-L215
    (*ARG_REGEX)
        .find_iter(&*proccode)
        .map(|s| s.as_str().to_string().trim().to_string())
        .filter(|s| s.as_str().starts_with('%'))
        .map(|s| s[..2].to_string())
        .map(|s| {
            Ok(match s.as_str() {
                "%n" => IrType::Number,
                "%s" => IrType::String,
                "%b" => IrType::Boolean,
                other => hq_bug!("invalid proccode arg \"{other}\" found"),
            })
        })
        .collect()
}

impl Proc {
    pub fn from_proccode(
        proccode: Box<str>,
        blocks: &BlockMap,
        target: Rc<Target>,
        expect_warp: bool,
    ) -> HQResult<Self> {
        let arg_types = arg_types_from_proccode(proccode.clone())?;
        let Some(prototype_block) = blocks.values().find(|block| {
            let Some(info) = block.block_info() else {
                return false;
            };
            info.opcode == BlockOpcode::procedures_prototype
                && (match info.mutation.mutations.get("proccode") {
                    Some(serde_json::Value::String(ref s)) => **s == *proccode,
                    _ => false,
                })
        }) else {
            hq_bad_proj!("no prototype found for {proccode} in")
        };
        let Some(def_block) = blocks.get(
            &prototype_block
                .block_info()
                .unwrap()
                .parent
                .clone()
                .ok_or(make_hq_bad_proj!("prototype block without parent"))?,
        ) else {
            hq_bad_proj!("no definition found for {proccode}")
        };
        if let Some(warp_val) = prototype_block
            .block_info()
            .unwrap()
            .mutation
            .mutations
            .get("warp")
        {
            let warp = match warp_val {
                serde_json::Value::Bool(w) => *w,
                serde_json::Value::String(wstr) => match wstr.as_str() {
                    "true" => true,
                    "false" => false,
                    _ => hq_bad_proj!("unexpected string for warp mutation for {proccode}"),
                },
                _ => hq_bad_proj!("bad type for warp mutation for {proccode}"),
            };
            if warp != expect_warp {
                hq_bad_proj!("proc call warp does not equal definition warp for {proccode}")
            }
        } else {
            hq_bad_proj!("missing warp mutation on procedures_definition for {proccode}")
        };
        let arg_ids = match prototype_block
            .block_info()
            .unwrap()
            .mutation
            .mutations
            .get("argumentids")
            .ok_or(make_hq_bad_proj!(
                "missing argumentids mutation for {proccode}"
            ))? {
            serde_json::Value::Array(values) => values
                .iter()
                .map(|val| match val {
                    serde_json::Value::String(s) => Ok(Into::<Box<str>>::into(s.clone())),
                    _ => hq_bad_proj!("non-string argumentids member in {proccode}"),
                })
                .collect::<HQResult<Box<[_]>>>()?,
            _ => hq_bad_proj!("non-array argumentids for {proccode}"),
        };
        let context = ProcedureContext {
            warp: expect_warp,
            arg_types,
            arg_ids,
            target: Rc::clone(&target),
        };
        let step_context = StepContext {
            target,
            proc_context: Some(context.clone()),
        };
        let first_step = match &def_block.block_info().unwrap().next {
            Some(next_id) => Step::from_block(
                blocks
                    .get(next_id)
                    .ok_or(make_hq_bad_proj!("specified next block does not exist"))?,
                blocks,
                step_context,
            )?,
            None => Step::new(step_context, Box::new([])),
        };
        Ok(Proc {
            first_step: Box::new(first_step),
            context,
        })
    }
}

pub type ProcMap = IndexMap<Box<str>, Rc<Proc>>;

#[derive(Clone, Default)]
pub struct ProcRegistry(RefCell<ProcMap>);

impl ProcRegistry {
    pub fn new() -> Self {
        ProcRegistry(RefCell::new(Default::default()))
    }

    pub(crate) fn get_map(&self) -> &RefCell<ProcMap> {
        &self.0
    }

    /// get the `Proc` for the specified proccode, creating it if it doesn't already exist
    pub fn proc(
        &self,
        proccode: Box<str>,
        blocks: &BlockMap,
        target: Rc<Target>,
        expect_warp: bool,
    ) -> HQResult<Rc<Proc>> {
        Ok(Rc::clone(
            self.get_map()
                .borrow_mut()
                .entry(proccode.clone())
                .or_insert(Rc::new(Proc::from_proccode(
                    proccode,
                    blocks,
                    target,
                    expect_warp,
                )?)),
        ))
    }
}
