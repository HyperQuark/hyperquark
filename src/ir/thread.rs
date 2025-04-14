use super::blocks::NextBlocks;
use super::{Event, IrProject, Step, StepContext, Target};
use crate::prelude::*;
use crate::sb3::{Block, BlockMap, BlockOpcode};

#[derive(Clone, Debug)]
pub struct Thread {
    event: Event,
    first_step: Rc<Step>,
    target: Weak<Target>,
}

impl Thread {
    pub fn event(&self) -> Event {
        self.event
    }

    pub fn first_step(&self) -> &Rc<Step> {
        &self.first_step
    }

    pub fn target(&self) -> Weak<Target> {
        Weak::clone(&self.target)
    }

    /// tries to construct a thread from a top-level block.
    /// Returns Ok(None) if the top-level block is not a valid event or if there is no next block.
    pub fn try_from_top_block(
        block: &Block,
        blocks: &BlockMap,
        target: Weak<Target>,
        project: Weak<IrProject>,
    ) -> HQResult<Option<Self>> {
        let Some(block_info) = block.block_info() else {
            return Ok(None);
        };

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
        let next_id = match &block_info.next {
            Some(next) => next,
            None => return Ok(None),
        };
        let next = blocks
            .get(next_id)
            .ok_or(make_hq_bug!("block not found in BlockMap"))?;
        let first_step = Step::from_block(
            next,
            next_id.clone(),
            blocks,
            StepContext {
                target: Weak::clone(&target),
                proc_context: None,
                warp: false, // steps from top blocks are never warped
            },
            Weak::clone(&project),
            NextBlocks::new(true),
        )?;
        Ok(Some(Thread {
            event,
            first_step,
            target,
        }))
    }
}
