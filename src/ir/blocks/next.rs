use crate::instructions::{HqYieldFields, IrOpcode, YieldMode};
use crate::ir::{InlinedStep, MaybeInlinedStep, Step, StepContext, StepIndex};
use crate::prelude::*;
use crate::sb3::{Block, BlockInfo};
use crate::wasm::WasmFlags;

#[derive(Clone, Debug)]
pub enum NextBlock {
    ID(Box<str>),
    Step(Step),
    StepIndex(StepIndex),
}

#[derive(Clone, Debug)]
pub struct NextBlockInfo {
    pub yield_first: bool,
    pub block: NextBlock,
}

/// contains a vector of next blocks, as well as information on how to proceed when
/// there are no next blocks: true => terminate the thread, false => do nothing
/// (useful for e.g. loop bodies, or for non-stack blocks)
#[derive(Clone, Debug)]
pub struct NextBlocks(Vec<NextBlockInfo>, bool);

impl NextBlocks {
    pub const fn new(terminating: bool) -> Self {
        Self(vec![], terminating)
    }

    pub const fn terminating(&self) -> bool {
        self.1
    }

    pub fn extend_with_inner(&self, new: NextBlockInfo) -> Self {
        let mut cloned = self.0.clone();
        cloned.push(new);
        Self(cloned, self.terminating())
    }

    pub fn pop_inner(self) -> (Option<NextBlockInfo>, Self) {
        let terminating = self.terminating();
        let mut vec = self.0;
        let popped = vec.pop();
        (popped, Self(vec, terminating))
    }
}

/// Generates the next step to go to, based off of the block info, and the `NextBlocks`
/// passed to it.
///
/// Returns a `MaybeInlinedStep` along with a `bool` indicating if the step should yield
/// first.
fn generate_next_step(
    block_info: &BlockInfo,
    blocks: &BTreeMap<Box<str>, Block>,
    context: &StepContext,
    final_next_blocks: NextBlocks,
    flags: &WasmFlags,
) -> HQResult<(MaybeInlinedStep, bool)> {
    let (next_block, yield_first, outer_next_blocks) = if let Some(ref next_block) = block_info.next
    {
        (
            Some(NextBlock::ID(next_block.clone())),
            false,
            final_next_blocks,
        )
    } else if let (Some(next_block_info), popped_next_blocks) =
        final_next_blocks.clone().pop_inner()
    {
        (
            Some(next_block_info.block),
            next_block_info.yield_first,
            popped_next_blocks,
        )
    } else {
        (None, false, final_next_blocks)
    };
    let next_step = match next_block {
        Some(NextBlock::ID(id)) => {
            let Some(next_block_block) = blocks.get(&id) else {
                hq_bad_proj!("next block doesn't exist")
            };
            MaybeInlinedStep::Undetermined(Step::from_block(
                next_block_block,
                id,
                blocks,
                context,
                &context.target().project(),
                outer_next_blocks,
                false,
                flags,
            )?)
        }
        Some(NextBlock::Step(step)) => MaybeInlinedStep::Undetermined(step),
        Some(NextBlock::StepIndex(step_index)) => MaybeInlinedStep::NonInlined(step_index),
        None => MaybeInlinedStep::Undetermined(Step::new_terminating(
            context.clone(),
            context.target().project(),
            false,
        )),
    };
    Ok((next_step, yield_first))
}

pub fn generate_next_step_inlined(
    block_info: &BlockInfo,
    blocks: &BTreeMap<Box<str>, Block>,
    context: &StepContext,
    final_next_blocks: NextBlocks,
    flags: &WasmFlags,
) -> HQResult<InlinedStep> {
    let (next_step, yield_first) =
        generate_next_step(block_info, blocks, context, final_next_blocks, flags)?;
    Ok(match next_step {
        MaybeInlinedStep::Inlined(step) => {
            if yield_first {
                let next_step_index = step
                    .try_borrow()?
                    .clone_to_non_inlined(&context.target().project())?;
                Rc::new(RefCell::new(Step::new(
                    None,
                    context.clone(),
                    vec![IrOpcode::hq_yield(HqYieldFields {
                        mode: YieldMode::Schedule(next_step_index),
                    })],
                    context.target().project(),
                    false,
                )))
            } else {
                step
            }
        }
        MaybeInlinedStep::NonInlined(step_index) => {
            if yield_first {
                Rc::new(RefCell::new(Step::new(
                    None,
                    context.clone(),
                    vec![IrOpcode::hq_yield(HqYieldFields {
                        mode: YieldMode::Schedule(step_index),
                    })],
                    context.target().project(),
                    false,
                )))
            } else {
                let mut step = context
                    .project()?
                    .steps()
                    .try_borrow()?
                    .get(step_index.0)
                    .ok_or_else(|| make_hq_bug!("step index out of bounds"))?
                    .try_borrow()?
                    .clone();
                step.make_inlined();
                Rc::new(RefCell::new(step))
            }
        }
        MaybeInlinedStep::Undetermined(mut step) => {
            if yield_first {
                let step_index = step.clone_to_non_inlined(&context.target().project())?;
                Rc::new(RefCell::new(Step::new(
                    None,
                    context.clone(),
                    vec![IrOpcode::hq_yield(HqYieldFields {
                        mode: YieldMode::Schedule(step_index),
                    })],
                    context.target().project(),
                    false,
                )))
            } else {
                step.make_inlined();
                Rc::new(RefCell::new(step))
            }
        }
    })
}

/// Generates a `StepIndex` for the next step of the given block, based on the
/// block info and the given `NextSteps`. The generated step must not be inlined
/// at a later stage, as it may cause reference cycles.
pub fn generate_next_step_non_inlined(
    block_info: &BlockInfo,
    blocks: &BTreeMap<Box<str>, Block>,
    context: &StepContext,
    final_next_blocks: NextBlocks,
    flags: &WasmFlags,
) -> HQResult<StepIndex> {
    let (next_step, _yield_first) =
        generate_next_step(block_info, blocks, context, final_next_blocks, flags)?;
    match next_step {
        MaybeInlinedStep::Inlined(step) => step
            .try_borrow()?
            .clone_to_non_inlined(&context.target().project()),
        MaybeInlinedStep::NonInlined(step_index) => Ok(step_index),
        MaybeInlinedStep::Undetermined(step) => {
            step.clone_to_non_inlined(&context.target().project())
        }
    }
}
