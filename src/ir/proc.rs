use super::blocks::NextBlocks;
use super::context::StepContext;
use super::{Step, Target as IrTarget, Type as IrType};
use crate::prelude::*;
use crate::sb3::{Block, BlockMap, BlockOpcode, Target as Sb3Target};
use core::cell::Ref;
use lazy_regex::{lazy_regex, Lazy};
use regex::Regex;

#[derive(Clone, Debug, PartialEq)]
pub enum PartialStep {
    None,
    StartedCompilation,
    Finished(Rc<Step>),
}

impl PartialStep {
    pub fn is_finished(&self) -> bool {
        matches!(self, PartialStep::Finished(_))
    }
}

#[derive(Clone, Debug)]
pub struct ProcContext {
    arg_types: Box<[IrType]>,
    arg_ids: Box<[Box<str>]>,
    arg_names: Box<[Box<str>]>,
    target: Weak<IrTarget>,
}

impl ProcContext {
    pub fn arg_ids(&self) -> &[Box<str>] {
        &self.arg_ids
    }

    pub fn arg_names(&self) -> &[Box<str>] {
        &self.arg_names
    }

    pub fn arg_types(&self) -> &[IrType] {
        &self.arg_types
    }

    pub fn target(&self) -> Weak<IrTarget> {
        Weak::clone(&self.target)
    }
}

#[derive(Clone, Debug)]
pub struct Proc {
    /// whether a procedure is 'run without screen refresh' - a procedure can be warped
    /// even without this condition holding true, if a procedure higher up in the call
    /// stack is warped.
    always_warped: bool,
    non_warped_first_step: RefCell<PartialStep>,
    warped_first_step: RefCell<PartialStep>,
    first_step_id: Option<Box<str>>,
    proccode: Box<str>,
    context: ProcContext,
}

impl Proc {
    pub fn non_warped_first_step(&self) -> HQResult<Ref<PartialStep>> {
        Ok(self.non_warped_first_step.try_borrow()?)
    }

    pub fn warped_first_step(&self) -> HQResult<Ref<PartialStep>> {
        Ok(self.warped_first_step.try_borrow()?)
    }

    pub fn first_step_id(&self) -> &Option<Box<str>> {
        &self.first_step_id
    }

    pub fn always_warped(&self) -> bool {
        self.always_warped
    }

    pub fn context(&self) -> &ProcContext {
        &self.context
    }

    pub fn proccode(&self) -> &str {
        &self.proccode
    }
}

static ARG_REGEX: Lazy<Regex> = lazy_regex!(r#"[^\\]%[nbs]"#);

fn arg_types_from_proccode(proccode: Box<str>) -> Result<Box<[IrType]>, HQError> {
    // https://github.com/scratchfoundation/scratch-blocks/blob/abbfe9/blocks_vertical/procedures.js#L207-L215
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
    fn string_vec_mutation(
        mutations: &BTreeMap<Box<str>, serde_json::Value>,
        id: &str,
    ) -> HQResult<Box<[Box<str>]>> {
        Ok(
            match mutations
                .get(id)
                .ok_or(make_hq_bad_proj!("missing {id} mutation"))?
            {
                serde_json::Value::Array(values) => values
                    .iter()
                    .map(|val| match val {
                        serde_json::Value::String(s) => Ok(s.clone().into_boxed_str()),
                        _ => hq_bad_proj!("non-string {id} member in"),
                    })
                    .collect::<HQResult<Box<[_]>>>()?,
                serde_json::Value::String(string_arr) => {
                    // let mut string_arr = string_arr.clone();
                    string_arr
                        .strip_prefix("[\"")
                        .ok_or(make_hq_bug!("malformed {id} array"))?
                        .strip_suffix("\"]")
                        .ok_or(make_hq_bug!("malformed {id} array"))?
                        .split("\",\"")
                        .map(Box::from)
                        .collect::<Box<[_]>>()
                }
                _ => hq_bad_proj!("non-array {id}"),
            },
        )
    }

    pub fn from_prototype(
        prototype: &Block,
        blocks: &BlockMap,
        target: Weak<IrTarget>,
    ) -> HQResult<Rc<Self>> {
        hq_assert!(prototype
            .block_info()
            .is_some_and(|info| info.opcode == BlockOpcode::procedures_prototype));
        let mutations = &prototype.block_info().unwrap().mutation.mutations;
        let serde_json::Value::String(proccode) = mutations.get("proccode").ok_or(
            make_hq_bad_proj!("missing proccode on procedures_prototype"),
        )?
        else {
            hq_bad_proj!("proccode wasn't a string");
        };
        let arg_types = arg_types_from_proccode(proccode.as_str().into())?;
        let Some(def_block) = blocks.get(
            &prototype
                .block_info()
                .unwrap()
                .parent
                .clone()
                .ok_or(make_hq_bad_proj!("prototype block without parent"))?,
        ) else {
            hq_bad_proj!("no definition block found for {proccode}")
        };
        let first_step_id = def_block
            .block_info()
            .ok_or(make_hq_bad_proj!("special block where normal def expected"))?
            .next
            .clone();
        let Some(warp_val) = mutations.get("warp") else {
            hq_bad_proj!("missing warp mutation on procedures_definition for {proccode}")
        };
        let warp = match warp_val {
            serde_json::Value::Bool(w) => *w,
            serde_json::Value::String(wstr) => match wstr.as_str() {
                "true" => true,
                "false" => false,
                _ => hq_bad_proj!("unexpected string for warp mutation for {proccode}"),
            },
            _ => hq_bad_proj!("bad type for warp mutation for {proccode}"),
        };
        let arg_ids = Proc::string_vec_mutation(mutations, "argumentids")?;
        let arg_names = Proc::string_vec_mutation(mutations, "argumentnames")?;
        let context = ProcContext {
            arg_types,
            arg_ids,
            arg_names,
            target,
        };
        Ok(Rc::new(Proc {
            proccode: proccode.as_str().into(),
            always_warped: warp,
            non_warped_first_step: RefCell::new(PartialStep::None),
            warped_first_step: RefCell::new(PartialStep::None),
            first_step_id,
            context,
        }))
    }

    pub fn compile_warped(&self, blocks: &BTreeMap<Box<str>, Block>) -> HQResult<()> {
        {
            if *self.non_warped_first_step()? != PartialStep::None {
                return Ok(());
            }
        }
        {
            *self.warped_first_step.try_borrow_mut()? = PartialStep::StartedCompilation;
        }
        let step_context = StepContext {
            warp: true,
            proc_context: Some(self.context.clone()),
            target: Weak::clone(&self.context.target),
        };
        let step = match self.first_step_id {
            None => Rc::new(Step::new(
                None,
                step_context.clone(),
                vec![],
                step_context.project()?,
            )),
            Some(ref id) => {
                let block = blocks.get(id).ok_or(make_hq_bad_proj!(
                    "procedure's first step block doesn't exist"
                ))?;
                Step::from_block(
                    block,
                    id.clone(),
                    blocks,
                    step_context.clone(),
                    step_context.project()?,
                    NextBlocks::NothingAtAll,
                )?
            }
        };
        *self.warped_first_step.try_borrow_mut()? = PartialStep::Finished(step);
        Ok(())
    }
}

pub type ProcMap = BTreeMap<Box<str>, Rc<Proc>>;

pub fn procs_from_target(sb3_target: &Sb3Target, ir_target: Rc<IrTarget>) -> HQResult<()> {
    let mut proc_map = ir_target.procedures_mut()?;
    for block in sb3_target.blocks.values() {
        let Block::Normal { block_info, .. } = block else {
            continue;
        };
        if block_info.opcode != BlockOpcode::procedures_prototype {
            continue;
        }
        let proc = Proc::from_prototype(block, &sb3_target.blocks, Rc::downgrade(&ir_target))?;
        let proccode = proc.proccode();
        proc_map.insert(proccode.into(), proc);
    }
    Ok(())
}
