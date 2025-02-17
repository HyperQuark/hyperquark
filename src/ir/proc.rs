use super::{IrProject, RcStep, Step, StepContext, Target, Type as IrType};
use crate::prelude::*;
use crate::registry::MapRegistry;
use crate::sb3::{BlockMap, BlockOpcode};
use lazy_regex::{lazy_regex, Lazy};
use regex::Regex;

#[derive(Clone, Debug)]
pub struct ProcedureContext {
    arg_ids: Box<[Box<str>]>,
    arg_types: Box<[IrType]>,
    warp: bool,
    target: Weak<Target>,
}

impl ProcedureContext {
    pub fn arg_ids(&self) -> &Box<[Box<str>]> {
        &self.arg_ids
    }

    pub fn arg_types(&self) -> &Box<[IrType]> {
        &self.arg_types
    }

    pub fn warp(&self) -> bool {
        self.warp
    }

    pub fn target(&self) -> Weak<Target> {
        Weak::clone(&self.target)
    }
}

#[derive(Clone)]
pub struct Proc {
    first_step: RcStep,
    context: ProcedureContext,
}

impl Proc {
    pub fn first_step(&self) -> &RcStep {
        &self.first_step
    }

    pub fn context(&self) -> &ProcedureContext {
        &self.context
    }
}

static ARG_REGEX: Lazy<Regex> = lazy_regex!(r#"[^\\]%[nbs]"#);

fn arg_types_from_proccode(proccode: Box<str>) -> Result<Box<[IrType]>, HQError> {
    // https://github.com/scratchfoundation/scratch-blocks/blob/abbfe93136fef57fdfb9a077198b0bc64726f012/blocks_vertical/procedures.js#L207-L215
    (*ARG_REGEX)
        .find_iter(&proccode)
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
        target: Weak<Target>,
        expect_warp: bool,
        project: Weak<IrProject>,
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
            target: Weak::clone(&target),
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
                next_id.clone(),
                blocks,
                step_context,
                project,
            )?,
            None => RcStep::new(Rc::new(Step::new(None, step_context, vec![], project))),
        };
        Ok(Proc {
            first_step,
            context,
        })
    }
}

pub type ProcRegistry = MapRegistry<Box<str>, Rc<Proc>>;

impl ProcRegistry {
    /// get the `Proc` for the specified proccode, creating it if it doesn't already exist
    pub fn proc(
        &self,
        proccode: Box<str>,
        blocks: &BlockMap,
        target: Weak<Target>,
        expect_warp: bool,
        project: Weak<IrProject>,
    ) -> HQResult<Rc<Proc>> {
        let idx = self.register(
            proccode.clone(),
            Rc::new(Proc::from_proccode(
                proccode,
                blocks,
                target,
                expect_warp,
                project,
            )?),
        )?;
        Ok(Rc::clone(
            self.registry()
                .try_borrow()?
                .get_index(idx)
                .ok_or(make_hq_bug!(
                    "recently inserted proc not found in ProcRegistry"
                ))?
                .1,
        ))
    }
}
