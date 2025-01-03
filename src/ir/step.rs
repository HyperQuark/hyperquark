use super::blocks;
use super::StepContext;
use crate::instructions::IrOpcode;
use crate::prelude::*;
use crate::sb3::{Block, BlockMap};

#[derive(Clone)]
pub struct Step {
    context: StepContext,
    opcodes: Box<[IrOpcode]>,
    inlined: RefCell<bool>
}

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

    pub fn set_inlined(&self, inlined: bool) {
        *self.inlined.borrow_mut() = inlined;
    }

    pub fn new(context: StepContext, opcodes: Box<[IrOpcode]>) -> Self {
        Step { context, opcodes, inlined: RefCell::new(false) }
    }

    pub fn from_block(block: &Block, blocks: &BlockMap, context: StepContext) -> HQResult<Self> {
        Ok(Step::new(context, blocks::from_block(block, blocks)?))
    }
}
