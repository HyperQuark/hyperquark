use super::{NextBlock, NextBlockInfo, NextBlocks, from_block, generate_next_step_inlined};
use crate::instructions::{
    ControlIfElseFields, ControlLoopFields, DataSetvariabletoFields, DataVariableFields,
    HqCastFields, HqTextFields, HqYieldFields, IrOpcode, YieldMode,
};
use crate::ir::{IrProject, RcVar, Step, StepContext, Target, Type as IrType};
use crate::prelude::*;
use crate::sb3::{Block, BlockArrayOrId, BlockInfo, Input, VarVal};
use crate::wasm::WasmFlags;

#[expect(clippy::too_many_arguments, reason = "too many arguments!")]
// TODO: put these arguments into a struct?
pub fn generate_loop(
    warp: bool,
    should_break: &mut bool,
    block_info: &BlockInfo,
    blocks: &BTreeMap<Box<str>, Block>,
    context: &StepContext,
    final_next_blocks: NextBlocks,
    first_condition_instructions: Option<Vec<IrOpcode>>,
    condition_instructions: Vec<IrOpcode>,
    pre_body_instructions: Option<Vec<IrOpcode>>,
    flip_if: bool,
    setup_instructions: Vec<IrOpcode>,
    flags: &WasmFlags,
) -> HQResult<Vec<IrOpcode>> {
    let substack_id = match block_info.inputs.get("SUBSTACK") {
        Some(
            Input::NoShadow(_, Some(substack_input)) | Input::Shadow(_, Some(substack_input), _),
        ) => {
            let BlockArrayOrId::Id(id) = substack_input else {
                hq_bad_proj!("malformed SUBSTACK input")
            };
            Some(id)
        }
        _ => None,
    };

    let substack_block = if let Some(id) = substack_id {
        Some(
            blocks
                .get(id)
                .ok_or_else(|| make_hq_bad_proj!("SUBSTACK block doesn't seem to exist"))?,
        )
    } else {
        None
    };
    if warp {
        // TODO: can this be expressed in the same way as non-warping loops,
        // just with yield_first: false?
        let substack_blocks = if let Some(block) = substack_block {
            from_block(
                block,
                blocks,
                context,
                &context.target().project(),
                NextBlocks::new(false),
                flags,
            )?
        } else {
            vec![]
        };
        let substack_step = Rc::new(RefCell::new(Step::new(
            None,
            context.clone(),
            substack_blocks,
            context.target().project(),
            false,
        )));
        let condition_step = Rc::new(RefCell::new(Step::new(
            None,
            context.clone(),
            condition_instructions,
            context.target().project(),
            false,
        )));
        let first_condition_step = first_condition_instructions.map(|instrs| {
            Rc::new(RefCell::new(Step::new(
                None,
                context.clone(),
                instrs,
                context.target().project(),
                false,
            )))
        });
        let pre_body_step = pre_body_instructions.map(|instrs| {
            Rc::new(RefCell::new(Step::new(
                None,
                context.clone(),
                instrs,
                context.target().project(),
                false,
            )))
        });
        Ok(setup_instructions
            .into_iter()
            .chain(vec![IrOpcode::control_loop(ControlLoopFields {
                first_condition: first_condition_step,
                condition: condition_step,
                body: substack_step,
                pre_body: pre_body_step,
                flip_if,
            })])
            .collect())
    } else {
        *should_break = true;
        let next_step =
            generate_next_step_inlined(block_info, blocks, context, final_next_blocks, flags)?;
        let project = context.project()?;
        let mut condition_step = Step::new(
            None,
            context.clone(),
            condition_instructions.clone(),
            context.target().project(),
            true,
        );
        let substack_step = Rc::new(RefCell::new(Step::new(
            None,
            context.clone(),
            vec![],
            context.target().project(),
            false,
        )));
        condition_step
            .opcodes_mut()
            .push(IrOpcode::control_if_else(ControlIfElseFields {
                branch_if: Rc::clone(if flip_if { &next_step } else { &substack_step }),
                branch_else: Rc::clone(if flip_if { &substack_step } else { &next_step }),
            }));
        let condition_step_index = project.new_owned_step(condition_step)?;
        if let Some(pre_body_blocks) = pre_body_instructions {
            substack_step
                .try_borrow_mut()?
                .opcodes_mut()
                .extend(pre_body_blocks);
        }
        if let Some(block) = substack_block {
            let substack_blocks = from_block(
                block,
                blocks,
                context,
                &context.target().project(),
                NextBlocks::new(false).extend_with_inner(NextBlockInfo {
                    yield_first: true,
                    block: NextBlock::StepIndex(condition_step_index),
                }),
                flags,
            )?;
            substack_step
                .try_borrow_mut()?
                .opcodes_mut()
                .extend(substack_blocks);
        } else {
            substack_step
                .try_borrow_mut()?
                .opcodes_mut()
                .push(IrOpcode::hq_yield(HqYieldFields {
                    mode: YieldMode::Schedule(condition_step_index),
                }));
        }
        Ok(setup_instructions
            .into_iter()
            .chain(first_condition_instructions.map_or(condition_instructions, |instrs| instrs))
            .chain(vec![IrOpcode::control_if_else(ControlIfElseFields {
                branch_if: Rc::clone(if flip_if { &next_step } else { &substack_step }),
                branch_else: Rc::clone(if flip_if { &substack_step } else { &next_step }),
            })])
            .collect())
    }
}

#[expect(clippy::too_many_arguments, reason = "will fix later. maybe.")]
pub fn generate_if_else(
    if_block: (&Block, Box<str>),
    maybe_else_block: Option<(&Block, Box<str>)>,
    block_info: &BlockInfo,
    final_next_blocks: &NextBlocks,
    blocks: &BTreeMap<Box<str>, Block>,
    context: &StepContext,
    should_break: &mut bool,
    flags: &WasmFlags,
) -> HQResult<Vec<IrOpcode>> {
    if !context.warp {
        let this_project = context.project()?;
        let dummy_project = Rc::new(IrProject::new(
            this_project.global_variables().clone(),
            this_project.global_lists().clone(),
            Box::from(this_project.broadcasts()),
            0,
            vec![],
        ));
        let dummy_target = Rc::new(Target::new(
            false,
            context.target().variables().clone(),
            context.target().lists().clone(),
            Rc::downgrade(&dummy_project),
            RefCell::new(context.target().procedures()?.clone()),
            0,
            context.target().costumes().into(),
        ));
        dummy_project
            .targets()
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .insert("".into(), Rc::clone(&dummy_target));
        let dummy_context = StepContext {
            target: Rc::clone(&dummy_target),
            ..context.clone()
        };
        let dummy_if_step = Step::from_block(
            if_block.0,
            if_block.1.clone(),
            blocks,
            &dummy_context,
            &Rc::downgrade(&dummy_project),
            NextBlocks::new(false),
            false,
            flags,
        )?;
        let dummy_else_step = if let Some((else_block, else_block_id)) = maybe_else_block.clone() {
            Step::from_block(
                else_block,
                else_block_id,
                blocks,
                &dummy_context,
                &Rc::downgrade(&dummy_project),
                NextBlocks::new(false),
                false,
                flags,
            )?
        } else {
            Step::new_empty(
                Rc::downgrade(&dummy_project),
                false,
                Rc::clone(&dummy_target),
            )
        };
        if dummy_if_step.does_yield() || dummy_else_step.does_yield() {
            // TODO: ideally if only one branch yields then we'd duplicate the next step and put one
            // version inline after the branch, and the other tagged on in the substep's NextBlocks
            // as usual, to allow for extra variable type optimisations.
            #[expect(
                clippy::option_if_let_else,
                reason = "map_or_else alternative is too complex"
            )]
            let (next_block, next_blocks) = if let Some(ref next_block) = block_info.next {
                (
                    Some(NextBlock::ID(next_block.clone())),
                    final_next_blocks.extend_with_inner(NextBlockInfo {
                        yield_first: false,
                        block: NextBlock::ID(next_block.clone()),
                    }),
                )
            } else if let (Some(next_block_info), _) = final_next_blocks.clone().pop_inner() {
                (Some(next_block_info.block), final_next_blocks.clone())
            } else {
                (
                    None,
                    final_next_blocks.clone(), // preserve termination behaviour
                )
            };
            let final_if_step = {
                Step::from_block(
                    if_block.0,
                    if_block.1,
                    blocks,
                    context,
                    &context.target().project(),
                    next_blocks.clone(),
                    false,
                    flags,
                )?
            };
            let final_else_step = if let Some((else_block, else_block_id)) = maybe_else_block {
                Step::from_block(
                    else_block,
                    else_block_id,
                    blocks,
                    context,
                    &context.target().project(),
                    next_blocks,
                    false,
                    flags,
                )?
            } else {
                let opcode = match next_block {
                    Some(NextBlock::ID(id)) => {
                        let next_block = blocks
                            .get(&id)
                            .ok_or_else(|| make_hq_bad_proj!("missing next block"))?;
                        vec![IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Inline(Rc::new(RefCell::new(Step::from_block(
                                next_block,
                                id.clone(),
                                blocks,
                                context,
                                &context.target().project(),
                                next_blocks,
                                false,
                                flags,
                            )?))),
                        })]
                    }
                    Some(NextBlock::Step(mut step)) => {
                        step.make_inlined();
                        vec![IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Inline(Rc::new(RefCell::new(step))),
                        })]
                    }
                    Some(NextBlock::StepIndex(step_index)) => {
                        let mut step = context
                            .project()?
                            .steps()
                            .try_borrow()?
                            .get(step_index.0)
                            .ok_or_else(|| make_hq_bug!("step index out of bounds"))?
                            .try_borrow()?
                            .clone();
                        step.make_inlined();
                        vec![IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Inline(Rc::new(RefCell::new(step))),
                        })]
                    }
                    None => {
                        if next_blocks.terminating() {
                            vec![IrOpcode::hq_yield(HqYieldFields {
                                mode: YieldMode::None,
                            })]
                        } else {
                            vec![]
                        }
                    }
                };
                Step::new(
                    None,
                    context.clone(),
                    opcode,
                    context.target().project(),
                    false,
                )
            };
            *should_break = true;
            return Ok(vec![IrOpcode::control_if_else(ControlIfElseFields {
                branch_if: Rc::new(RefCell::new(final_if_step)),
                branch_else: Rc::new(RefCell::new(final_else_step)),
            })]);
        }
    }
    let final_if_step = Step::from_block(
        if_block.0,
        if_block.1,
        blocks,
        context,
        &context.target().project(),
        NextBlocks::new(false),
        false,
        flags,
    )?;
    let final_else_step = if let Some((else_block, else_block_id)) = maybe_else_block {
        Step::from_block(
            else_block,
            else_block_id,
            blocks,
            context,
            &context.target().project(),
            NextBlocks::new(false),
            false,
            flags,
        )?
    } else {
        Step::new(
            None,
            context.clone(),
            vec![],
            context.target().project(),
            false,
        )
    };
    Ok(vec![IrOpcode::control_if_else(ControlIfElseFields {
        branch_if: Rc::new(RefCell::new(final_if_step)),
        branch_else: Rc::new(RefCell::new(final_else_step)),
    })])
}

pub fn generate_exhaustive_string_comparison<I, S, F>(
    string_source: I,
    instruction: F,
    fallback: Vec<IrOpcode>,
    context: &StepContext,
    project: &Weak<IrProject>,
    flags: &WasmFlags,
) -> HQResult<Vec<IrOpcode>>
where
    I: IntoIterator<Item = S>,
    S: Into<Box<str>> + Clone,
    F: Fn(Box<str>) -> IrOpcode,
{
    let var = RcVar::new(IrType::String, &VarVal::String("".into()), None, flags)?;
    Ok(vec![
        IrOpcode::hq_cast(HqCastFields(IrType::String)),
        IrOpcode::data_setvariableto(DataSetvariabletoFields {
            var: RefCell::new(var.clone()),
            local_write: RefCell::new(true),
            first_write: RefCell::new(true),
        }),
    ]
    .into_iter()
    .chain(
        string_source
            .into_iter()
            .fold(
                Rc::new(RefCell::new(Step::new(
                    None,
                    context.clone(),
                    fallback,
                    Weak::clone(project),
                    false,
                ))),
                |branch_else, string| {
                    let branch_if = Rc::new(RefCell::new(Step::new(
                        None,
                        context.clone(),
                        vec![instruction(string.clone().into())],
                        Weak::clone(project),
                        false,
                    )));
                    Rc::new(RefCell::new(Step::new(
                        None,
                        context.clone(),
                        vec![
                            IrOpcode::data_variable(DataVariableFields {
                                var: RefCell::new(var.clone()),
                                local_read: RefCell::new(true),
                            }),
                            IrOpcode::hq_text(HqTextFields(string.into())),
                            IrOpcode::operator_equals, // todo: this should be a case-sensitive comparison
                            IrOpcode::control_if_else(ControlIfElseFields {
                                branch_if,
                                branch_else,
                            }),
                        ],
                        Weak::clone(project),
                        false,
                    )))
                },
            )
            .try_borrow()?
            .opcodes()
            .clone(),
    )
    .collect())
}
