//! A procedure here is actually a function; although scratch procedures can't return values,
//! we can (we do this to improve variable type analysis). Procedures can return as many values
//! as we want, because we can use the WASM multi-value proposal.

use super::blocks::NextBlocks;
use super::context::StepContext;
use super::{Step, Target as IrTarget};
use crate::ir::{ProcContext, RcVar};
use crate::prelude::*;
use crate::sb3::{Block, BlockArrayOrId, BlockMap, BlockOpcode, Input, Target as Sb3Target};
use crate::wasm::WasmFlags;
use core::cell::{Ref, RefMut};
use lazy_regex::{Lazy, lazy_regex};
use regex::Regex;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PartialStep {
    None,
    StartedCompilation,
    Finished(Rc<Step>),
}

#[derive(Clone, Debug)]
pub struct SpecificProc {
    first_step: RefCell<PartialStep>,
    arg_vars: Rc<RefCell<Vec<RcVar>>>,
    return_vars: Rc<RefCell<Vec<RcVar>>>,
}

impl SpecificProc {
    pub fn first_step(&self) -> HQResult<Ref<'_, PartialStep>> {
        Ok(self.first_step.try_borrow()?)
    }

    #[must_use]
    pub fn arg_vars(&self) -> Rc<RefCell<Vec<RcVar>>> {
        Rc::clone(&self.arg_vars)
    }

    #[must_use]
    pub fn return_vars(&self) -> Rc<RefCell<Vec<RcVar>>> {
        Rc::clone(&self.return_vars)
    }

    fn proc_context(&self, arg_names: Box<[Box<str>]>) -> ProcContext {
        ProcContext {
            arg_vars: Rc::clone(&self.arg_vars),
            ret_vars: Rc::clone(&self.return_vars),
            arg_names,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Proc {
    #[expect(clippy::struct_field_names, reason = "i like it")]
    warped_specific_proc: RefCell<Option<SpecificProc>>,
    #[expect(clippy::struct_field_names, reason = "i like it")]
    nonwarped_specific_proc: RefCell<Option<SpecificProc>>,
    first_step_id: Option<Box<str>>,
    proccode: Box<str>,
    debug: bool,
    arg_ids: Box<[Box<str>]>,
    arg_names: Box<[Box<str>]>,
    target: Rc<IrTarget>,
    /// whether a procedure is 'run without screen refresh'.
    always_warped: bool,
}

impl Proc {
    pub fn warped_specific_proc(&self) -> Ref<'_, Option<SpecificProc>> {
        self.warped_specific_proc.borrow()
    }

    pub fn nonwarped_specific_proc(&self) -> Ref<'_, Option<SpecificProc>> {
        self.nonwarped_specific_proc.borrow()
    }

    pub fn proccode(&self) -> &str {
        &self.proccode
    }

    #[must_use]
    pub fn arg_ids(&self) -> &[Box<str>] {
        &self.arg_ids
    }

    #[must_use]
    pub fn arg_names(&self) -> &[Box<str>] {
        &self.arg_names
    }

    #[must_use]
    pub const fn always_warped(&self) -> bool {
        self.always_warped
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
            .map(|_| RcVar::new_empty())
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
        #[expect(
            clippy::unwrap_used,
            reason = "previously asserted that block_info is Some"
        )]
        let Some((parent_id, def_block)) =
            (if let Some(parent_id) = prototype.block_info().unwrap().parent.as_ref() {
                Some((
                    parent_id.clone(),
                    blocks.get(parent_id).ok_or_else(|| {
                        make_hq_bad_proj!("non-existant parent id on prototype block")
                    })?,
                ))
            } else {
                blocks
                    .iter()
                    .map(|(id, block)| (id.clone(), block))
                    .try_find(|(_id, block)| {
                        let Some(block_info) = block.block_info() else {
                            return Ok(false);
                        };
                        if block_info.opcode != BlockOpcode::procedures_definition {
                            return Ok(false);
                        }
                        let Some(custom_block_input) = block_info.inputs.get("custom_block") else {
                            hq_bad_proj!("missing custom_block input");
                        };
                        let (Input::NoShadow(_, Some(block_arr_id))
                        | Input::Shadow(_, Some(block_arr_id), _)) = custom_block_input
                        else {
                            hq_bad_proj!("nullish block array/id for custom_block input");
                        };
                        let BlockArrayOrId::Id(custom_block_id) = block_arr_id else {
                            hq_bad_proj!("unexpected array-like input for custom_block input");
                        };
                        let Some(this_proc_prototype) = blocks.get(custom_block_id) else {
                            hq_bad_proj!("non-existant block id specified for custom_block input");
                        };
                        Ok(this_proc_prototype == prototype)
                    })?
            })
        else {
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
        Ok(Rc::new(Self {
            proccode: proccode.as_str().into(),
            warped_specific_proc: RefCell::new(None),
            nonwarped_specific_proc: RefCell::new(None),
            first_step_id,
            debug,
            arg_ids,
            arg_names,
            target,
            always_warped: warp,
        }))
    }

    fn new_specific_proc(&self) -> SpecificProc {
        SpecificProc {
            first_step: RefCell::new(PartialStep::None),
            arg_vars: arg_vars_from_proccode(&self.proccode),
            return_vars: Rc::new(RefCell::new(vec![])),
        }
    }

    pub fn compile_warped(
        &self,
        blocks: &BTreeMap<Box<str>, Block>,
        flags: &WasmFlags,
    ) -> HQResult<()> {
        self.compile(
            blocks,
            flags,
            || Ok(self.warped_specific_proc.try_borrow()?),
            || Ok(self.warped_specific_proc.try_borrow_mut()?),
            true,
        )
    }

    pub fn compile_nonwarped(
        &self,
        blocks: &BTreeMap<Box<str>, Block>,
        flags: &WasmFlags,
    ) -> HQResult<()> {
        self.compile(
            blocks,
            flags,
            || Ok(self.nonwarped_specific_proc.try_borrow()?),
            || Ok(self.nonwarped_specific_proc.try_borrow_mut()?),
            false,
        )
    }

    pub fn compile<'a, F, G>(
        &self,
        blocks: &BTreeMap<Box<str>, Block>,
        flags: &WasmFlags,
        specific_proc: F,
        specific_proc_mut: G,
        warp: bool,
    ) -> HQResult<()>
    where
        F: Fn() -> HQResult<Ref<'a, Option<SpecificProc>>>,
        G: Fn() -> HQResult<RefMut<'a, Option<SpecificProc>>>,
    {
        {
            if specific_proc()?
                .as_ref()
                .is_some_and(|p| p.first_step().is_ok_and(|s| *s != PartialStep::None))
            {
                return Ok(());
            }
        }

        {
            let maybe_specific_proc = specific_proc()?.clone();

            if maybe_specific_proc.is_some() {
                #[expect(clippy::unwrap_used, reason = "already checked that it is_some")]
                *specific_proc_mut()?
                    .as_ref()
                    .unwrap()
                    .first_step
                    .try_borrow_mut()? = PartialStep::StartedCompilation;
            } else {
                let new_specific_proc = self.new_specific_proc();
                *new_specific_proc.first_step.try_borrow_mut()? = PartialStep::StartedCompilation;
                *specific_proc_mut()? = Some(new_specific_proc);
            }
        }

        let specific_proc_option = specific_proc()?;
        #[expect(
            clippy::unwrap_used,
            reason = "must be Some as we assign to it if None"
        )]
        let some_specific_proc = specific_proc_option.as_ref().unwrap();

        let step_context = StepContext {
            warp,
            proc_context: Some(some_specific_proc.proc_context(self.arg_names.clone())),
            target: Rc::clone(&self.target),
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
                    NextBlocks::new(!warp),
                    true,
                    flags,
                )?
            }
        };

        #[expect(
            clippy::unwrap_used,
            reason = "must be Some as we assigned to it if None"
        )]
        *specific_proc()?
            .as_ref()
            .unwrap()
            .first_step
            .try_borrow_mut()? = PartialStep::Finished(step);

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

impl fmt::Display for SpecificProc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let first_step = self.first_step.borrow();
        let first_step_str = match *first_step {
            PartialStep::Finished(ref step) => step.id(),
            PartialStep::StartedCompilation | PartialStep::None => "none",
        };
        let arg_vars_cell = &self.arg_vars();
        let arg_vars = format!(
            "[{}]",
            RefCell::borrow(arg_vars_cell)
                .iter()
                .map(|var| format!("{var}"))
                .join(", ")
        );
        let ret_vars_cell = &self.return_vars();
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
            "first_step": "{first_step_str}",
            "arg_vars": {arg_vars},
            "return_vars": {ret_vars}
        }}"#
        )
    }
}

impl fmt::Display for Proc {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let proccode = self.proccode();
        let always_warped = self.always_warped();
        let warped_specific_proc = if let Some(ref nwsp) = self.warped_specific_proc().clone() {
            format!("{nwsp}")
        } else {
            "null".to_string()
        };
        let nonwarped_specific_proc = if let Some(ref nwsp) = self.nonwarped_specific_proc().clone()
        {
            format!("{nwsp}")
        } else {
            "null".to_string()
        };
        write!(
            f,
            r#"{{
            "proccode": "{proccode}",
            "always_warped": {always_warped},
            "warped_specific_proc": {warped_specific_proc},
            "nonwarped_specific_proc": {nonwarped_specific_proc}
        }}"#
        )
    }
}
