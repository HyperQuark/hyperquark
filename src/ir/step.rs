use super::StepContext;
use crate::instructions::IrOpcode;
use crate::prelude::*;
use crate::sb3::{Block, BlockMap};

#[derive(Clone)]
pub struct Step {
    context: StepContext,
    opcodes: Box<[IrOpcode]>,
}

impl Step {
    pub fn context(&self) -> &StepContext {
        &self.context
    }

    pub fn opcodes(&self) -> &[IrOpcode] {
        self.opcodes.borrow()
    }

    pub fn new(context: StepContext, opcodes: Box<[IrOpcode]>) -> Self {
        Step { context, opcodes }
    }

    pub fn from_block(_block: &Block, _blocks: &BlockMap, _context: StepContext) -> HQResult<Self> {
        hq_todo!()
    }
}
