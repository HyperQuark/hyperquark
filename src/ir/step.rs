use super::blocks::{self, NextBlocks};
use super::{IrProject, StepContext};
use crate::instructions::{ControlIfElseFields, HqYieldFields, IrOpcode, YieldMode};
use crate::ir::Target;
use crate::prelude::*;
use crate::sb3::{Block, BlockMap};
use crate::wasm::WasmFlags;
use core::cell::RefMut;
use uuid::Uuid;

#[derive(Debug)]
pub struct Step {
    context: StepContext,
    opcodes: RefCell<Vec<IrOpcode>>,
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
    pub const fn context(&self) -> &StepContext {
        &self.context
    }

    pub const fn opcodes(&self) -> &RefCell<Vec<IrOpcode>> {
        &self.opcodes
    }

    pub const fn used_non_inline(&self) -> bool {
        self.used_non_inline
    }

    pub const fn id(&self) -> &str {
        &self.id
    }

    pub fn project(&self) -> Weak<IrProject> {
        Weak::clone(&self.project)
    }

    pub fn clone(&self, used_non_inline: bool) -> HQResult<Rc<Self>> {
        Self::new_rc(
            None,
            self.context.clone(),
            self.opcodes.try_borrow()?.clone(),
            &Weak::clone(&self.project),
            used_non_inline,
        )
    }

    fn new(
        id: Option<Box<str>>,
        context: StepContext,
        opcodes: Vec<IrOpcode>,
        project: Weak<IrProject>,
        used_non_inline: bool,
    ) -> Self {
        Self {
            id: id.unwrap_or_else(|| Uuid::new_v4().to_string().into()),
            context,
            opcodes: RefCell::new(opcodes),
            used_non_inline,
            project,
        }
    }

    pub fn new_rc(
        id: Option<Box<str>>,
        context: StepContext,
        opcodes: Vec<IrOpcode>,
        project: &Weak<IrProject>,
        used_non_inline: bool,
    ) -> HQResult<Rc<Self>> {
        let step = Rc::new(Self::new(
            id,
            context,
            opcodes,
            Weak::clone(project),
            used_non_inline,
        ));
        if used_non_inline {
            project
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak"))?
                .steps()
                .try_borrow_mut()
                .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
                .insert(Rc::clone(&step));
        } else {
            project
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak"))?
                .inlined_steps()
                .try_borrow_mut()
                .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
                .insert(Rc::clone(&step));
        }
        Ok(step)
    }

    /// Creates a totally empty noop step. This should not be used outside of wasm module generation.
    pub fn new_empty(
        project: &Weak<IrProject>,
        used_non_inline: bool,
        target: Rc<Target>,
    ) -> HQResult<Rc<Self>> {
        Self::new_rc(
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

    pub fn new_terminating(
        context: StepContext,
        project: &Weak<IrProject>,
        used_non_inline: bool,
    ) -> HQResult<Rc<Self>> {
        Self::new_rc(
            None,
            context,
            vec![IrOpcode::hq_yield(HqYieldFields {
                mode: YieldMode::None,
            })],
            project,
            used_non_inline,
        )
    }

    pub fn opcodes_mut(&self) -> HQResult<RefMut<'_, Vec<IrOpcode>>> {
        self.opcodes
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))
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
    ) -> HQResult<Rc<Self>> {
        if used_non_inline
            && let Some(existing_step) = project
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak"))?
                .steps()
                .try_borrow()
                .map_err(|_| make_hq_bug!("couldn't immutably borrow cell"))?
                .iter()
                .find(|step| step.id == block_id)
        {
            crate::log(
                format!("step from_block already exists! (id: {block_id:?}); returning early")
                    .as_str(),
            );
            return Ok(Rc::clone(existing_step));
        }
        let id = if used_non_inline {
            Some(block_id)
        } else {
            None
        };
        Self::new_rc(
            id,
            context.clone(),
            blocks::from_block(block, blocks, context, project, final_next_blocks, flags)?,
            project,
            used_non_inline,
        )
    }

    pub fn does_yield(&self) -> HQResult<bool> {
        for opcode in &*self.opcodes().try_borrow()? {
            if opcode.requests_screen_refresh() {
                return Ok(true);
            }
            #[expect(clippy::wildcard_enum_match_arm, reason = "too many variants to match")]
            match opcode {
                IrOpcode::hq_yield(HqYieldFields {
                    mode: YieldMode::Schedule(_),
                }) => return Ok(true),
                IrOpcode::control_if_else(ControlIfElseFields {
                    branch_else,
                    branch_if,
                }) => {
                    if branch_if.does_yield()? || branch_else.does_yield()? {
                        return Ok(true);
                    }
                }
                _ => (),
            }
        }
        Ok(false)
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
        let opcodes = self
            .opcodes()
            .borrow()
            .iter()
            .map(|op| format!("{op}"))
            .join(", ");
        write!(
            f,
            r#"{{
        "id": "{id}",
        "opcodes": [{opcodes}]
    }}"#
        )
    }
}
