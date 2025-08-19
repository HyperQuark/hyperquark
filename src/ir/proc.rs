//! A procedure here is actually a function; although scratch procedures can't return values,
//! we can (we do this to improve variable type analysis). Procedures can return as many values
//! as we want, because we can use the WASM multi-value proposal.

use super::blocks::NextBlocks;
use super::context::StepContext;
use super::{Step, Target as IrTarget, Type as IrType};
use crate::ir::RcVar;
use crate::prelude::*;
use crate::sb3::{Block, BlockMap, BlockOpcode, Target as Sb3Target};
use crate::wasm::WasmFlags;
use core::cell::Ref;
use lazy_regex::{Lazy, lazy_regex};
use regex::Regex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartialStep {
    None,
    StartedCompilation,
    Finished(Rc<Step>),
}

#[derive(Clone, Debug)]
pub struct ProcContext {
    arg_vars: Rc<RefCell<Vec<RcVar>>>,
    return_vars: Rc<RefCell<Vec<RcVar>>>,
    arg_ids: Box<[Box<str>]>,
    arg_names: Box<[Box<str>]>,
    target: Rc<IrTarget>,
    /// whether a procedure is 'run without screen refresh'
    always_warped: bool,
}

impl ProcContext {
    pub fn arg_ids(&self) -> &[Box<str>] {
        &self.arg_ids
    }

    pub fn arg_names(&self) -> &[Box<str>] {
        &self.arg_names
    }

    pub fn arg_vars(&self) -> Rc<RefCell<Vec<RcVar>>> {
        Rc::clone(&self.arg_vars)
    }

    pub fn return_vars(&self) -> Rc<RefCell<Vec<RcVar>>> {
        Rc::clone(&self.return_vars)
    }

    pub const fn always_warped(&self) -> bool {
        self.always_warped
    }
}

#[derive(Clone, Debug)]
pub struct Proc {
    first_step: RefCell<PartialStep>,
    first_step_id: Option<Box<str>>,
    proccode: Box<str>,
    context: ProcContext,
    debug: bool,
}

impl Proc {
    pub fn first_step(&self) -> HQResult<Ref<'_, PartialStep>> {
        Ok(self.first_step.try_borrow()?)
    }

    pub const fn context(&self) -> &ProcContext {
        &self.context
    }

    pub fn proccode(&self) -> &str {
        &self.proccode
    }
}

static ARG_REGEX: Lazy<Regex> = lazy_regex!(r#"[^\\]%[nbs]"#);

fn arg_vars_from_proccode(proccode: &str) -> Rc<RefCell<Vec<RcVar>>> {
    // based off of https://github.com/scratchfoundation/scratch-blocks/blob/abbfe9/blocks_vertical/procedures.js#L207-L215
    Rc::new(RefCell::new(
        (*ARG_REGEX)
            .find_iter(proccode)
            .map(|s| s.as_str().to_string().trim().to_string())
            .filter(|s| s.as_str().starts_with('%'))
            .map(|_| RcVar::new(IrType::none(), crate::sb3::VarVal::Bool(false)))
            .collect(),
    ))
}

impl Proc {
    fn string_vec_mutation(
        mutations: &BTreeMap<Box<str>, serde_json::Value>,
        id: &str,
    ) -> HQResult<Box<[Box<str>]>> {
        Ok(
            match mutations
                .get(id)
                .ok_or_else(|| make_hq_bad_proj!("missing {id} mutation"))?
            {
                serde_json::Value::Array(values) => values
                    .iter()
                    .map(|val| {
                        if let serde_json::Value::String(s) = val {
                            Ok(s.clone().into_boxed_str())
                        } else {
                            hq_bad_proj!("non-string {id} member in")
                        }
                    })
                    .collect::<HQResult<Box<[_]>>>()?,
                serde_json::Value::String(string_arr) => {
                    if string_arr == "[]" {
                        Box::new([])
                    } else {
                        string_arr
                            .strip_prefix("[\"")
                            .ok_or_else(|| make_hq_bug!("malformed {id} array"))?
                            .strip_suffix("\"]")
                            .ok_or_else(|| make_hq_bug!("malformed {id} array"))?
                            .split("\",\"")
                            .map(Box::from)
                            .collect::<Box<[_]>>()
                    }
                }
                serde_json::Value::Null
                | serde_json::Value::Bool(_)
                | serde_json::Value::Number(_)
                | serde_json::Value::Object(_) => hq_bad_proj!("non-array {id}"),
            },
        )
    }

    pub fn from_prototype(
        prototype: &Block,
        blocks: &BlockMap,
        target: Rc<IrTarget>,
        sb3_target: &Sb3Target,
    ) -> HQResult<Rc<Self>> {
        hq_assert!(
            prototype
                .block_info()
                .is_some_and(|info| info.opcode == BlockOpcode::procedures_prototype)
        );
        #[expect(
            clippy::unwrap_used,
            reason = "previously asserted that block_info is Some"
        )]
        let mutations = &prototype.block_info().unwrap().mutation.mutations;
        let serde_json::Value::String(proccode) = mutations
            .get("proccode")
            .ok_or_else(|| make_hq_bad_proj!("missing proccode on procedures_prototype"))?
        else {
            hq_bad_proj!("proccode wasn't a string");
        };
        let arg_vars = arg_vars_from_proccode(proccode.as_str());
        #[expect(
            clippy::unwrap_used,
            reason = "previously asserted that block_info is Some"
        )]
        let parent_id = prototype
            .block_info()
            .unwrap()
            .parent
            .clone()
            .ok_or_else(|| make_hq_bad_proj!("prototype block without parent"))?;
        let Some(def_block) = blocks.get(&parent_id) else {
            hq_bad_proj!("no definition block found for {proccode}")
        };
        let debug = sb3_target.comments.clone().iter().any(|(_id, comment)| {
            matches!(comment.block_id.clone(), Some(d) if d == parent_id)
                && *comment.text.clone() == *"hq-dbg"
        });
        let first_step_id = def_block
            .block_info()
            .ok_or_else(|| make_hq_bad_proj!("special block where normal def expected"))?
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
            serde_json::Value::Null
            | serde_json::Value::Number(_)
            | serde_json::Value::Array(_)
            | serde_json::Value::Object(_) => {
                hq_bad_proj!("bad type for warp mutation for {proccode}")
            }
        };
        let arg_ids = Self::string_vec_mutation(mutations, "argumentids")?;
        let arg_names = Self::string_vec_mutation(mutations, "argumentnames")?;
        let context = ProcContext {
            arg_vars,
            arg_ids,
            arg_names,
            target,
            return_vars: Rc::new(RefCell::new(vec![])),
            always_warped: warp,
        };
        Ok(Rc::new(Self {
            proccode: proccode.as_str().into(),
            first_step: RefCell::new(PartialStep::None),
            first_step_id,
            context,
            debug,
        }))
    }

    pub fn compile_warped(
        &self,
        blocks: &BTreeMap<Box<str>, Block>,
        flags: &WasmFlags,
    ) -> HQResult<()> {
        hq_assert!(
            self.context().always_warped(),
            "tried to call compile_warped on a non-warped proc"
        );
        {
            if *self.first_step()? != PartialStep::None {
                return Ok(());
            }
        }
        {
            *self.first_step.try_borrow_mut()? = PartialStep::StartedCompilation;
        }
        let step_context = StepContext {
            warp: true,
            proc_context: Some(self.context.clone()),
            target: Rc::clone(&self.context.target),
            debug: self.debug,
        };
        let step = match self.first_step_id {
            None => Step::new_rc(
                None,
                step_context.clone(),
                vec![],
                &step_context.target().project(),
                true,
            )?,
            Some(ref id) => {
                let block = blocks.get(id).ok_or_else(|| {
                    make_hq_bad_proj!("procedure's first step block doesn't exist")
                })?;
                Step::from_block(
                    block,
                    id.clone(),
                    blocks,
                    &step_context,
                    &step_context.target().project(),
                    NextBlocks::new(false),
                    true,
                    flags,
                )?
            }
        };
        *self.first_step.try_borrow_mut()? = PartialStep::Finished(step);
        Ok(())
    }
}

pub type ProcMap = BTreeMap<Box<str>, Rc<Proc>>;

pub fn procs_from_target(sb3_target: &Sb3Target, ir_target: &Rc<IrTarget>) -> HQResult<()> {
    let mut proc_map = ir_target.procedures_mut()?;
    for block in sb3_target.blocks.values() {
        let Block::Normal { block_info, .. } = block else {
            continue;
        };
        if block_info.opcode != BlockOpcode::procedures_prototype {
            continue;
        }
        let proc =
            Proc::from_prototype(block, &sb3_target.blocks, Rc::clone(ir_target), sb3_target)?;
        let proccode = proc.proccode();
        proc_map.insert(proccode.into(), proc);
    }
    Ok(())
}

impl fmt::Display for Proc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let proccode = self.proccode();
        let always_warped = self.context().always_warped();
        let partial_step = self.first_step().map_err(|_| fmt::Error)?.clone();
        let step = match partial_step.borrow() {
            PartialStep::Finished(step) => step.id(),
            PartialStep::StartedCompilation | PartialStep::None => "none",
        };
        let arg_vars_cell = &self.context().arg_vars();
        let arg_vars = format!(
            "[{}]",
            RefCell::borrow(arg_vars_cell)
                .iter()
                .map(|var| format!("{var}"))
                .join(", ")
        );
        let ret_vars_cell = &self.context().return_vars();
        let ret_vars = format!(
            "[{}]",
            RefCell::borrow(ret_vars_cell)
                .iter()
                .map(|var| format!("{var}"))
                .join(", ")
        );
        write!(
            f,
            r#"{{
            "proccode": "{proccode}",
            "always_warped": {always_warped},
            "first_step": "{step}",
            "arg_vars": {arg_vars},
            "return_vars": {ret_vars}
        }}"#
        )
    }
}
