use super::blocks::NextBlocks;
use super::{Event, IrProject, Step, StepContext, Target};
use crate::prelude::*;
use crate::sb3::{Block, BlockMap, BlockOpcode};
use crate::wasm::WasmFlags;

#[derive(Clone, Debug)]
pub struct Thread {
    event: Event,
    first_step: Rc<Step>,
}

impl Thread {
    pub const fn event(&self) -> Event {
        self.event
    }

    pub const fn first_step(&self) -> &Rc<Step> {
        &self.first_step
    }

    /// tries to construct a thread from a top-level block.
    /// Returns Ok(None) if the top-level block is not a valid event or if there is no next block.
    pub fn try_from_top_block(
        block: &Block,
        blocks: &BlockMap,
        target: &Weak<Target>,
        project: &Weak<IrProject>,
        debug: bool,
        flags: &WasmFlags
    ) -> HQResult<Option<Self>> {
        let Some(block_info) = block.block_info() else {
            return Ok(None);
        };
        #[expect(clippy::wildcard_enum_match_arm, reason = "too many variants to match")]
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
        let Some(next_id) = &block_info.next else {
            return Ok(None);
        };
        let next = blocks
            .get(next_id)
            .ok_or_else(|| make_hq_bug!("block not found in BlockMap"))?;
        let first_step = Step::from_block(
            next,
            next_id.clone(),
            blocks,
            &StepContext {
                target: Weak::clone(target),
                proc_context: None,
                warp: false, // steps from top blocks are never warped
                debug,
            },
            project,
            NextBlocks::new(true),
            true,
            flags
        )?;
        Ok(Some(Self {
            event,
            first_step,
        }))
    }
}

impl fmt::Display for Thread {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let event = self.event();
        let first_step = self.first_step().id();
        write!(
            f,
            r#"{{
            "event": "{event:?}",
            "first_step": "{first_step}",
        }}"#
        )
    }
}
