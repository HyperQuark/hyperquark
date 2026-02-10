use uuid::Uuid;

use super::blocks::{self, NextBlocks};
use super::{IrProject, StepContext};
use crate::instructions::{ControlIfElseFields, HqYieldFields, IrOpcode, YieldMode};
use crate::ir::{RcVar, Target, used_vars};
use crate::prelude::*;
use crate::sb3::{Block, BlockMap};
use crate::wasm::WasmFlags;

#[derive(Debug, Copy, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StepIndex(pub usize);

#[derive(Debug)]
pub struct Step {
    context: StepContext,
    opcodes: Vec<IrOpcode>,
    used_non_inline: bool,
    /// used for `Hash`. Should be obtained from a block in the `Step` where possible.
    id: Box<str>,
    project: Weak<IrProject>,
}

impl PartialEq for Step {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for Step {}

impl Step {
    #[must_use]
    pub const fn context(&self) -> &StepContext {
        &self.context
    }

    #[must_use]
    pub const fn opcodes(&self) -> &Vec<IrOpcode> {
        &self.opcodes
    }

    #[must_use]
    pub const fn used_non_inline(&self) -> bool {
        self.used_non_inline
    }

    #[must_use]
    pub const fn id(&self) -> &str {
        &self.id
    }

    #[must_use]
    pub fn project(&self) -> Weak<IrProject> {
        Weak::clone(&self.project)
    }

    #[must_use]
    pub fn new(
        id: Option<Box<str>>,
        context: StepContext,
        opcodes: Vec<IrOpcode>,
        project: Weak<IrProject>,
        used_non_inline: bool,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| Uuid::new_v4().to_string().into()),
            context,
            opcodes,
            used_non_inline,
            project,
        }
    }

    /// Creates a totally empty noop step. This should not be used outside of wasm module generation.
    #[must_use]
    pub fn new_empty(project: Weak<IrProject>, used_non_inline: bool, target: Rc<Target>) -> Self {
        Self::new(
            None,
            StepContext {
                target,
                warp: false,
                proc_context: None,
                debug: false,
            },
            vec![],
            project,
            used_non_inline,
        )
    }

    #[must_use]
    pub fn new_terminating(
        context: StepContext,
        project: Weak<IrProject>,
        used_non_inline: bool,
    ) -> Self {
        Self::new(
            None,
            context,
            vec![IrOpcode::hq_yield(HqYieldFields {
                mode: YieldMode::None,
            })],
            project,
            used_non_inline,
        )
    }

    #[must_use]
    pub fn new_poll_waiting_threads(context: StepContext, project: Weak<IrProject>) -> Self {
        Self::new(
            None,
            context.clone(),
            vec![
                IrOpcode::event_poll_waiting_threads,
                IrOpcode::control_if_else(ControlIfElseFields {
                    branch_if: Rc::new(RefCell::new(Self::new(
                        None,
                        context.clone(),
                        vec![],
                        Weak::clone(&project),
                        false,
                    ))),
                    branch_else: Rc::new(RefCell::new(Self::new_terminating(
                        context,
                        Weak::clone(&project),
                        false,
                    ))),
                }),
            ],
            project,
            true,
        )
    }

    #[must_use]
    pub fn new_poll_waiting_event(context: StepContext, project: Weak<IrProject>) -> Self {
        Self::new(
            None,
            context.clone(),
            vec![
                IrOpcode::hq_poll_waiting_event,
                IrOpcode::control_if_else(ControlIfElseFields {
                    branch_else: Rc::new(RefCell::new(Self::new(
                        None,
                        context.clone(),
                        vec![],
                        Weak::clone(&project),
                        false,
                    ))),
                    branch_if: Rc::new(RefCell::new(Self::new_terminating(
                        context,
                        Weak::clone(&project),
                        false,
                    ))),
                }),
            ],
            project,
            true,
        )
    }

    #[must_use]
    pub fn new_poll_timer(context: StepContext, project: Weak<IrProject>) -> Self {
        Self::new(
            None,
            context.clone(),
            vec![
                IrOpcode::control_get_thread_timeout,
                IrOpcode::sensing_timer,
                IrOpcode::operator_gt,
                IrOpcode::control_if_else(ControlIfElseFields {
                    branch_if: Rc::new(RefCell::new(Self::new(
                        None,
                        context.clone(),
                        vec![],
                        Weak::clone(&project),
                        false,
                    ))),
                    branch_else: Rc::new(RefCell::new(Self::new_terminating(
                        context,
                        Weak::clone(&project),
                        false,
                    ))),
                }),
            ],
            project,
            true,
        )
    }

    #[must_use]
    pub const fn opcodes_mut(&mut self) -> &mut Vec<IrOpcode> {
        &mut self.opcodes
    }

    pub fn from_block(
        block: &Block,
        block_id: Box<str>,
        blocks: &BlockMap,
        context: &StepContext,
        project: &Weak<IrProject>,
        final_next_blocks: NextBlocks,
        used_non_inline: bool,
        flags: &WasmFlags,
    ) -> HQResult<Self> {
        Ok(Self::new(
            Some(block_id),
            context.clone(),
            blocks::from_block(block, blocks, context, project, final_next_blocks, flags)?,
            Weak::clone(project),
            used_non_inline,
        ))
    }

    pub fn from_block_non_inlined(
        block: &Block,
        block_id: Box<str>,
        blocks: &BlockMap,
        context: &StepContext,
        project: &Weak<IrProject>,
        final_next_blocks: NextBlocks,
        flags: &WasmFlags,
    ) -> HQResult<StepIndex> {
        if let Some((existing_step, _)) = project
            .upgrade()
            .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak"))?
            .steps()
            .try_borrow()
            .map_err(|_| make_hq_bug!("couldn't immutably borrow cell"))?
            .iter()
            .find_position(|step| RefCell::borrow(step).id == block_id)
        {
            // crate::log(
            //     format!("step from_block already exists! (id: {block_id:?}); returning early")
            //         .as_str(),
            // );
            return Ok(StepIndex(existing_step));
        }
        let step = Self::from_block(
            block,
            block_id,
            blocks,
            context,
            project,
            final_next_blocks,
            true,
            flags,
        )?;
        let rc_project = project
            .upgrade()
            .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak"))?;
        rc_project.new_owned_step(step)
    }

    pub fn make_inlined(&mut self) {
        if !self.used_non_inline {
            return;
        }
        self.used_non_inline = false;
        self.id = Uuid::new_v4().to_string().into();
    }

    pub fn clone_to_non_inlined(&self, project: &Weak<IrProject>) -> HQResult<StepIndex> {
        let mut step = self.clone();
        step.used_non_inline = true;
        let rc_project = project
            .upgrade()
            .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak"))?;
        rc_project.new_owned_step(step)
    }

    #[must_use]
    pub fn does_yield(&self) -> bool {
        for opcode in self.opcodes() {
            if opcode.yields_to_next_step().is_some() {
                return true;
            }
            if opcode
                .inline_steps()
                .is_some_and(|steps| steps.iter().any(|step| RefCell::borrow(step).does_yield()))
            {
                return true;
            }
        }
        false
    }

    /// An iterator of variables that are global (in the WASM sense, not the scratch sense)
    /// in this step. Should be in a consistent order. Used for procedures.
    pub fn globally_scoped_variables(
        &self,
    ) -> HQResult<impl core::iter::DoubleEndedIterator<Item = RcVar> + Clone> {
        let target = self.context().target();
        let project = self
            .project()
            .upgrade()
            .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<IrProject>"))?;
        let global_vars = used_vars(project.global_variables());
        let target_vars = used_vars(target.variables());
        Ok(global_vars.into_iter().chain(target_vars))
    }

    pub fn globally_scoped_variables_num(&self) -> HQResult<usize> {
        let target = self.context().target();
        let project = self
            .project()
            .upgrade()
            .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<IrProject>"))?;
        let global_vars = used_vars(project.global_variables());
        let target_vars = used_vars(target.variables());
        Ok(global_vars.len() + target_vars.len())
    }
}

impl Clone for Step {
    fn clone(&self) -> Self {
        Self::new(
            None,
            self.context.clone(),
            self.opcodes.clone(),
            Weak::clone(&self.project),
            self.used_non_inline,
        )
    }
}

impl core::hash::Hash for Step {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl PartialOrd for Step {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Step {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.id().cmp(other.id())
    }
}

impl fmt::Display for Step {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let id = self.id();
        let opcodes = self.opcodes().iter().map(|op| format!("{op}")).join(", ");
        write!(
            f,
            r#"{{
        "id": "{id}",
        "opcodes": [{opcodes}]
    }}"#
        )
    }
}

pub type InlinedStep = Rc<RefCell<Step>>;

#[derive(Debug, Clone)]
pub enum MaybeInlinedStep {
    Inlined(InlinedStep),
    NonInlined(StepIndex),
    Undetermined(Step),
}
