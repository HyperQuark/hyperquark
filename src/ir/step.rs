use super::StepContext;
use crate::instructions::IrOpcode;
use crate::prelude::*;
use crate::sb3::{Block, BlockMap, BlockOpcode};

#[derive(Clone)]
pub struct Step {
    context: StepContext,
    opcodes: Box<[IrOpcode]>,
}

impl Step {
    pub fn new(context: StepContext, opcodes: Box<[IrOpcode]>) -> Self {
        Step { context, opcodes }
    }

    pub fn from_block(block: &Block, blocks: &BlockMap, context: StepContext) -> HQResult<Self> {
        hq_todo!()
    }
}
