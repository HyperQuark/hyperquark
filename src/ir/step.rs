use super::blocks;
use super::{IrProject, StepContext};
use crate::instructions::IrOpcode;
use crate::prelude::*;
use crate::sb3::{Block, BlockMap};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct Step {
    context: StepContext,
    opcodes: Vec<IrOpcode>,
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

    pub fn opcodes(&self) -> &Vec<IrOpcode> {
        &self.opcodes
    }

    pub fn inlined(&self) -> &RefCell<bool> {
        &self.inlined
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
            opcodes,
            inlined: RefCell::new(false),
            project,
        }
    }

    pub fn new_empty() -> Self {
        Step {
            id: "".into(),
            context: StepContext {
                target: Weak::new(),
                proc_context: None,
            },
            opcodes: vec![IrOpcode::hq_integer(crate::instructions::HqIntegerFields(
                0,
            ))],
            inlined: RefCell::new(false),
            project: Weak::new(),
        }
    }

    pub fn from_block(
        block: &Block,
        block_id: Box<str>,
        blocks: &BlockMap,
        context: StepContext,
        project: Weak<IrProject>,
    ) -> HQResult<Rc<Step>> {
        let step = Rc::new(Step::new(
            Some(block_id),
            context.clone(),
            blocks::from_block(block, blocks, &context)?,
            Weak::clone(&project),
        ));
        project
            .upgrade()
            .ok_or(make_hq_bug!("couldn't upgrade Weak"))?
            .steps()
            .try_borrow_mut()?
            .insert(Rc::clone(&step));
        Ok(step)
    }

    pub fn make_inlined(&self) -> HQResult<()> {
        if *self.inlined.try_borrow()? {
            return Ok(());
        };
        *self.inlined.try_borrow_mut()? = true;
        self.project
            .upgrade()
            .ok_or(make_hq_bug!("couldn't upgrade Weak"))?
            .inlined_steps()
            .try_borrow_mut()?
            .insert(
                self.project
                    .upgrade()
                    .ok_or(make_hq_bug!("couldn't upgrade Weak"))?
                    .steps()
                    .try_borrow_mut()?
                    .swap_take(self)
                    .ok_or(make_hq_bug!("step not in project's StepMap"))?,
            );
        Ok(())
    }
}

impl core::hash::Hash for Step {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}
