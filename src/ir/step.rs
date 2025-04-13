use super::blocks::{self, NextBlocks, StackMode};
use super::{IrProject, StepContext};
use crate::instructions::{HqYieldFields, IrOpcode, YieldMode};
use crate::prelude::*;
use crate::sb3::{Block, BlockMap};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Step {
    context: StepContext,
    opcodes: RefCell<Vec<IrOpcode>>,
    used_non_inline: RefCell<bool>,
    inline: RefCell<bool>,
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
    pub fn context(&self) -> &StepContext {
        &self.context
    }

    pub fn opcodes(&self) -> &RefCell<Vec<IrOpcode>> {
        &self.opcodes
    }

    pub fn used_non_inline(&self) -> &RefCell<bool> {
        &self.used_non_inline
    }

    pub fn inline(&self) -> &RefCell<bool> {
        &self.inline
    }

    pub fn project(&self) -> Weak<IrProject> {
        Weak::clone(&self.project)
    }

    pub fn new(
        id: Option<Box<str>>,
        context: StepContext,
        opcodes: Vec<IrOpcode>,
        project: Weak<IrProject>,
    ) -> Self {
        Step {
            id: id.unwrap_or_else(|| Uuid::new_v4().to_string().into()),
            context,
            opcodes: RefCell::new(opcodes),
            used_non_inline: RefCell::new(false),
            inline: RefCell::new(false),
            project,
        }
    }

    pub fn new_rc(
        id: Option<Box<str>>,
        context: StepContext,
        opcodes: Vec<IrOpcode>,
        project: Weak<IrProject>,
    ) -> HQResult<Rc<Self>> {
        let step = Rc::new(Step::new(id, context, opcodes, Weak::clone(&project)));
        project
            .upgrade()
            .ok_or(make_hq_bug!("couldn't upgrade Weak"))?
            .steps()
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .insert(Rc::clone(&step));
        Ok(step)
    }

    pub fn new_empty() -> Self {
        Step {
            id: "".into(),
            context: StepContext {
                target: Weak::new(),
                proc_context: None,
                warp: false, // this is a fairly arbitrary choice and doesn't matter at all
            },
            opcodes: RefCell::new(vec![]),
            used_non_inline: RefCell::new(false),
            inline: RefCell::new(false),
            project: Weak::new(),
        }
    }

    pub fn new_terminating(context: StepContext, project: Weak<IrProject>) -> HQResult<Rc<Step>> {
        const ID: &str = "__terminating_step_hopefully_this_id_wont_cause_any_clashes";
        let step = Rc::new(Step {
            id: ID.into(),
            context,
            opcodes: RefCell::new(vec![IrOpcode::hq_yield(HqYieldFields {
                mode: YieldMode::None,
            })]),
            used_non_inline: RefCell::new(false),
            inline: RefCell::new(false),
            project: Weak::clone(&project),
        });
        project
            .upgrade()
            .ok_or(make_hq_bug!("couldn't upgrade Weak"))?
            .steps()
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .insert(Rc::clone(&step));
        Ok(step)
    }

    pub fn opcodes_mut(&self) -> HQResult<core::cell::RefMut<Vec<IrOpcode>>> {
        self.opcodes
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))
    }

    pub fn from_block(
        block: &Block,
        block_id: Box<str>,
        blocks: &BlockMap,
        context: StepContext,
        project: Weak<IrProject>,
        final_next_blocks: NextBlocks,
    ) -> HQResult<Rc<Step>> {
        if let Some(existing_step) = project
            .upgrade()
            .ok_or(make_hq_bug!("couldn't upgrade Weak"))?
            .steps()
            .try_borrow()
            .map_err(|_| make_hq_bug!("couldn't immutably borrow cell"))?
            .iter()
            .find(|step| step.id == block_id)
        {
            crate::log("step from_block already exists!");
            return Ok(Rc::clone(existing_step));
        }
        let step = Rc::new(Step::new(
            Some(block_id),
            context.clone(),
            blocks::from_block(
                block,
                StackMode::Stack,
                blocks,
                &context,
                Weak::clone(&project),
                final_next_blocks,
            )?,
            Weak::clone(&project),
        ));
        project
            .upgrade()
            .ok_or(make_hq_bug!("couldn't upgrade Weak"))?
            .steps()
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .insert(Rc::clone(&step));

        Ok(step)
    }

    pub fn make_used_non_inline(&self) -> HQResult<()> {
        if *self.used_non_inline.try_borrow()? {
            return Ok(());
        };
        *self
            .used_non_inline
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))? = true;
        Ok(())
    }

    pub fn make_inlined(&self) -> HQResult<()> {
        if *self.inline.try_borrow()? {
            return Ok(());
        };
        *self
            .inline
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))? = true;
        Ok(())
    }
}

impl core::hash::Hash for Step {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
