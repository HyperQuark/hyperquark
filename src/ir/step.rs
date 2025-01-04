use super::blocks;
use super::{IrProject, StepContext};
use crate::instructions::IrOpcode;
use crate::prelude::*;
use crate::sb3::{Block, BlockMap};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Step {
    context: StepContext,
    opcodes: Box<[IrOpcode]>,
    /// is this step inlined? if not, its own function should be produced
    inlined: RefCell<bool>,
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

    pub fn opcodes(&self) -> &[IrOpcode] {
        self.opcodes.borrow()
    }

    pub fn inlined(&self) -> bool {
        *self.inlined.borrow()
    }

    pub fn project(&self) -> Weak<IrProject> {
        Weak::clone(&self.project)
    }

    pub fn new(
        id: Option<Box<str>>,
        context: StepContext,
        opcodes: Box<[IrOpcode]>,
        project: Weak<IrProject>,
    ) -> Self {
        Step {
            id: id.unwrap_or_else(|| Uuid::new_v4().to_string().into()),
            context,
            opcodes,
            inlined: RefCell::new(false),
            project,
        }
    }

    pub fn from_block(
        block: &Block,
        block_id: Box<str>,
        blocks: &BlockMap,
        context: StepContext,
        project: Weak<IrProject>,
    ) -> HQResult<RcStep> {
        let step = Rc::new(Step::new(
            Some(block_id),
            context,
            blocks::from_block(block, blocks)?,
            Weak::clone(&project),
        ));
        project
            .upgrade()
            .ok_or(make_hq_bug!("couldn't upgrade Weak"))?
            .steps()
            .borrow_mut()
            .insert(Rc::clone(&step));
        Ok(RcStep(step))
    }
}

impl core::hash::Hash for Step {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

#[derive(Clone, Debug)]
pub struct RcStep(Rc<Step>);

impl RcStep {
    pub fn new(rc: Rc<Step>) -> Self {
        RcStep(rc)
    }

    pub fn get_rc(&self) -> Rc<Step> {
        Rc::clone(&self.0)
    }

    pub fn make_inlined(&self) -> HQResult<()> {
        if *self.0.inlined.borrow() {
            return Ok(());
        };
        *self.0.inlined.borrow_mut() = true;
        self.0
            .project
            .upgrade()
            .ok_or(make_hq_bug!("couldn't upgrade Weak"))?
            .inlined_steps()
            .borrow_mut()
            .insert(
                self.0
                    .project
                    .upgrade()
                    .ok_or(make_hq_bug!("couldn't upgrade Weak"))?
                    .steps()
                    .borrow_mut()
                    .swap_take(&self.0)
                    .ok_or(make_hq_bug!("step not in project's StepMap"))?,
            );
        Ok(())
    }
}
