use super::{Event, Step, StepContext, TargetContext};
use crate::prelude::*;
use crate::sb3::{Block, BlockMap, BlockOpcode};

#[derive(Clone)]
pub struct Thread {
    event: Event,
    first_step: Box<Step>,
}

impl Thread {
    pub fn event(&self) -> Event {
        self.event
    }

    pub fn first_step(&self) -> &Step {
        self.first_step.borrow()
    }

    /// tries to construct a thread from a top-level block.
    /// Returns Ok(None) if the top-level block is not a valid event or if there is no next block.
    pub fn try_from_top_block(
        block: &Block,
        blocks: &BlockMap,
        target_context: TargetContext,
    ) -> HQResult<Option<Self>> {
        let block_info = block
            .block_info()
            .ok_or(make_hq_bug!("top-level block is a special block"))?;
        let event = match block_info.opcode {
            BlockOpcode::event_whenflagclicked => Event::FlagCLicked,
            BlockOpcode::event_whenbackdropswitchesto
            | BlockOpcode::event_whenbroadcastreceived
            | BlockOpcode::event_whengreaterthan
            | BlockOpcode::event_whenkeypressed
            | BlockOpcode::event_whenstageclicked
            | BlockOpcode::event_whenthisspriteclicked
            | BlockOpcode::event_whentouchingobject => {
                hq_todo!("unimplemented event {:?}", block_info.opcode)
            }
            _ => return Ok(None),
        };
        let next = blocks
            .get(match &block_info.next {
                Some(next) => next,
                None => return Ok(None),
            })
            .ok_or(make_hq_bug!("block not found in BlockMap"))?;
        Ok(Some(Thread {
            event,
            first_step: Box::new(Step::from_block(
                next,
                blocks,
                StepContext {
                    target_context,
                    proc_context: None,
                },
            )?),
        }))
    }
}
