mod cast;
mod control_flow;
mod inputs;
mod list_op;
mod next;
mod proc_arg;
mod special;

pub use cast::insert_casts;
use control_flow::{generate_exhaustive_string_comparison, generate_if_else, generate_loop};
use inputs::inputs;
use list_op::generate_list_index_op;
pub use next::NextBlocks;
use next::{NextBlock, NextBlockInfo, generate_next_step_inlined, generate_next_step_non_inlined};
use proc_arg::{ProcArgType, procedure_argument};
use special::from_special_block;

use super::context::StepContext;
use super::{IrProject, RcVar, Step, Type as IrType};
use crate::instructions::{
    ControlLoopFields, ControlWaitFields, DataAddtolistFields, DataDeletealloflistFields,
    DataDeleteoflistFields, DataInsertatlistFields, DataItemoflistFields, DataLengthoflistFields,
    DataListcontentsFields, DataReplaceitemoflistFields, DataSetvariabletoFields,
    DataTeevariableFields, DataVariableFields, DataVisvariableFields, EventBroadcastAndWaitFields,
    EventBroadcastFields, HqBooleanFields, HqCastFields, HqFloatFields, HqIntegerFields,
    HqTextFields, HqYieldFields, IrOpcode, LooksSayFields, LooksThinkFields,
    ProceduresCallNonwarpFields, ProceduresCallWarpFields, SensingAskandwaitFields, YieldMode,
};
use crate::prelude::*;
use crate::sb3::{
    Block, BlockArrayOrId, BlockInfo, BlockMap, BlockOpcode, Field as Sb3Field, VarVal,
};
use crate::wasm::WasmFlags;

pub fn from_block(
    block: &Block,
    blocks: &BlockMap,
    context: &StepContext,
    project: &Weak<IrProject>,
    final_next_blocks: NextBlocks,
    flags: &WasmFlags,
) -> HQResult<Vec<IrOpcode>> {
    let mut opcodes = match block {
        Block::Normal { block_info, .. } => from_normal_block(
            block_info,
            blocks,
            context,
            project,
            final_next_blocks,
            flags,
        )?
        .to_vec(),
        Block::Special(block_array) => vec![from_special_block(block_array, context, flags)?],
    };
    insert_casts(&mut opcodes, true, false)?;
    Ok(opcodes)
}

fn from_normal_block(
    block_info: &BlockInfo,
    blocks: &BlockMap,
    context: &StepContext,
    project: &Weak<IrProject>,
    final_next_blocks: NextBlocks,
    flags: &WasmFlags,
) -> HQResult<Box<[IrOpcode]>> {
    let mut curr_block = Some(block_info);
    let mut final_next_blocks = final_next_blocks;
    let mut opcodes = vec![];
    let mut should_break = false;
    while let Some(block_info) = curr_block {
        opcodes.append(
            &mut inputs(block_info, blocks, context, project, flags)?
                .into_iter()
                .chain(block_to_ir(
                    block_info,
                    blocks,
                    context,
                    project,
                    &final_next_blocks,
                    flags,
                    &mut should_break,
                )?)
                .collect(),
        );
        if should_break {
            break;
        }
        curr_block = if let Some(ref next_id) = block_info.next {
            let next_block = blocks
                .get(next_id)
                .ok_or_else(|| make_hq_bad_proj!("missing next block"))?;
            next_block.block_info()
        } else if let (Some(popped_next), new_next_blocks_stack) =
            final_next_blocks.clone().pop_inner()
        {
            match popped_next.block {
                NextBlock::ID(id) => {
                    let next_block = blocks
                        .get(&id)
                        .ok_or_else(|| make_hq_bad_proj!("missing next block"))?;
                    if (popped_next.yield_first) && !context.warp {
                        opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Schedule(Step::from_block_non_inlined(
                                next_block,
                                id.clone(),
                                blocks,
                                context,
                                project,
                                new_next_blocks_stack,
                                flags,
                            )?),
                        }));
                        None
                    } else {
                        final_next_blocks = new_next_blocks_stack;
                        next_block.block_info()
                    }
                }
                NextBlock::Step(mut step) => {
                    if popped_next.yield_first && !context.warp {
                        opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Schedule(context.project()?.new_owned_step(step)?),
                        }));
                    } else {
                        step.make_inlined();
                        opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Inline(Rc::new(RefCell::new(step))),
                        }));
                    }
                    None
                }
                NextBlock::StepIndex(step_index) => {
                    if popped_next.yield_first && !context.warp {
                        opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Schedule(step_index),
                        }));
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
                        opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Inline(Rc::new(RefCell::new(step))),
                        }));
                    }
                    None
                }
            }
        } else if final_next_blocks.terminating() {
            opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                mode: YieldMode::None,
            }));
            None
        } else {
            None
        }
    }
    Ok(opcodes.into_iter().collect())
}

fn block_to_ir(
    block_info: &BlockInfo,
    blocks: &BlockMap,
    context: &StepContext,
    project: &Weak<IrProject>,
    final_next_blocks: &NextBlocks,
    flags: &WasmFlags,
    should_break: &mut bool,
) -> HQResult<Vec<IrOpcode>> {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "too many opcodes to match individually"
    )]
    Ok(match &block_info.opcode {
        BlockOpcode::operator_add => vec![IrOpcode::operator_add],
        BlockOpcode::operator_subtract => vec![IrOpcode::operator_subtract],
        BlockOpcode::operator_multiply => vec![IrOpcode::operator_multiply],
        BlockOpcode::operator_divide => vec![IrOpcode::operator_divide],
        BlockOpcode::operator_mod => vec![IrOpcode::operator_modulo],
        BlockOpcode::motion_gotoxy => vec![IrOpcode::motion_gotoxy],
        BlockOpcode::motion_setx => vec![IrOpcode::motion_setx],
        BlockOpcode::motion_sety => vec![IrOpcode::motion_sety],
        BlockOpcode::motion_xposition => vec![IrOpcode::motion_xposition],
        BlockOpcode::motion_yposition => vec![IrOpcode::motion_yposition],
        BlockOpcode::motion_changexby => vec![
            IrOpcode::motion_xposition,
            IrOpcode::operator_add,
            IrOpcode::motion_setx,
        ],
        BlockOpcode::motion_changeyby => vec![
            IrOpcode::motion_yposition,
            IrOpcode::operator_add,
            IrOpcode::motion_sety,
        ],
        BlockOpcode::motion_movesteps => vec![
            // this is a really lazy implementation but wasm-opt should optimise it
            IrOpcode::hq_dup,
            IrOpcode::hq_float(HqFloatFields(90.0)),
            IrOpcode::motion_direction,
            IrOpcode::operator_subtract,
            IrOpcode::operator_cos,
            IrOpcode::operator_multiply,
            IrOpcode::motion_xposition,
            IrOpcode::operator_add,
            IrOpcode::hq_swap,
            IrOpcode::hq_float(HqFloatFields(90.0)),
            IrOpcode::motion_direction,
            IrOpcode::operator_subtract,
            IrOpcode::operator_sin,
            IrOpcode::operator_multiply,
            IrOpcode::motion_yposition,
            IrOpcode::operator_add,
            IrOpcode::motion_gotoxy,
        ],
        BlockOpcode::motion_direction => vec![IrOpcode::motion_direction],
        BlockOpcode::motion_pointindirection => {
            vec![IrOpcode::motion_pointindirection]
        }
        BlockOpcode::motion_turnright => vec![
            IrOpcode::motion_direction,
            IrOpcode::operator_add,
            IrOpcode::motion_pointindirection,
        ],
        BlockOpcode::motion_turnleft => vec![
            IrOpcode::motion_direction,
            IrOpcode::operator_subtract,
            IrOpcode::hq_integer(HqIntegerFields(-1)),
            IrOpcode::operator_multiply,
            IrOpcode::motion_pointindirection,
        ],
        BlockOpcode::looks_say => vec![IrOpcode::looks_say(LooksSayFields {
            debug: context.debug,
            target_idx: context.target().index(),
        })],
        BlockOpcode::looks_think => vec![IrOpcode::looks_think(LooksThinkFields {
            debug: context.debug,
            target_idx: context.target().index(),
        })],
        BlockOpcode::operator_join => vec![IrOpcode::operator_join],
        BlockOpcode::operator_length => vec![IrOpcode::operator_length],
        BlockOpcode::operator_contains => vec![IrOpcode::operator_contains],
        BlockOpcode::operator_letter_of => vec![IrOpcode::operator_letter_of],
        BlockOpcode::sensing_dayssince2000 => vec![IrOpcode::sensing_dayssince2000],
        BlockOpcode::sensing_keypressed => vec![IrOpcode::sensing_keypressed],
        BlockOpcode::sensing_keyoptions => {
            let (Sb3Field::Value((Some(val),)) | Sb3Field::ValueId(Some(val), _)) =
                block_info.fields.get("KEY_OPTION").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field KEY_OPTION")
                })?
            else {
                hq_bad_proj!("invalid project.json - missing value for KEY_OPTION field")
            };
            let VarVal::String(key_option) = val else {
                hq_bad_proj!("invalid project.json - non-string value for KEY_OPTION field")
            };
            vec![IrOpcode::hq_text(HqTextFields(key_option.clone()))]
        }
        BlockOpcode::sensing_timer => vec![IrOpcode::sensing_timer],
        BlockOpcode::sensing_mousex => vec![IrOpcode::sensing_mousex],
        BlockOpcode::sensing_mousey => vec![IrOpcode::sensing_mousey],
        BlockOpcode::sensing_mousedown => vec![IrOpcode::sensing_mousedown],
        BlockOpcode::sensing_answer => vec![IrOpcode::sensing_answer],
        BlockOpcode::sensing_resettimer => vec![IrOpcode::sensing_reset_timer],
        BlockOpcode::operator_lt => vec![IrOpcode::operator_lt],
        BlockOpcode::operator_gt => vec![IrOpcode::operator_gt],
        BlockOpcode::operator_equals => vec![IrOpcode::operator_equals],
        BlockOpcode::operator_not => vec![IrOpcode::operator_not],
        BlockOpcode::operator_and => vec![IrOpcode::operator_and],
        BlockOpcode::operator_or => vec![IrOpcode::operator_or],
        BlockOpcode::operator_round => vec![IrOpcode::operator_round],
        BlockOpcode::operator_mathop => {
            let (Sb3Field::Value((Some(val),)) | Sb3Field::ValueId(Some(val), _)) =
                block_info.fields.get("OPERATOR").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field OPERATOR")
                })?
            else {
                hq_bad_proj!("invalid project.json - missing value for OPERATOR field")
            };
            let VarVal::String(operator) = val else {
                hq_bad_proj!("invalid project.json - non-string value for OPERATOR field")
            };
            match operator.to_lowercase().as_str() {
                "abs" => vec![IrOpcode::operator_abs],
                "floor" => vec![IrOpcode::operator_floor],
                "ceiling" => vec![IrOpcode::operator_ceiling],
                "sqrt" => vec![IrOpcode::operator_sqrt],
                "sin" => vec![IrOpcode::operator_sin],
                "cos" => vec![IrOpcode::operator_cos],
                "tan" => vec![IrOpcode::operator_tan],
                "asin" => vec![IrOpcode::operator_asin],
                "acos" => vec![IrOpcode::operator_acos],
                "atan" => vec![IrOpcode::operator_atan],
                "ln" => vec![IrOpcode::operator_ln],
                "log" => vec![IrOpcode::operator_log],
                "e ^" => vec![IrOpcode::operator_exp],
                "10 ^" => vec![IrOpcode::operator_pow10],
                other => hq_bad_proj!("unknown mathop {}", other),
            }
        }
        BlockOpcode::operator_random => vec![IrOpcode::operator_random],
        BlockOpcode::event_broadcast_menu => {
            let Sb3Field::ValueId(val, _id) =
                block_info.fields.get("BROADCAST_OPTION").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field BROADCAST_OPTION")
                })?
            else {
                hq_bad_proj!(
                    "invalid project.json - missing broadcast name for BROADCAST_OPTION field"
                );
            };
            let VarVal::String(name) = val.clone().ok_or_else(|| {
                make_hq_bad_proj!(
                    "invalid project.json - null broadcast name for BROADCAST_OPTION field"
                )
            })?
            else {
                hq_bad_proj!("non-string broadcast name")
            };
            vec![IrOpcode::hq_text(HqTextFields(name))]
        }
        BlockOpcode::event_broadcast => generate_exhaustive_string_comparison(
            context.project()?.broadcasts().iter().cloned(),
            |broadcast| IrOpcode::event_broadcast(EventBroadcastFields(broadcast)),
            vec![],
            context,
            project,
            flags,
        )?,
        BlockOpcode::event_broadcastandwait => {
            let poll_step = context
                .project()?
                .new_owned_step(Step::new_poll_waiting_threads(
                    context.clone(),
                    Weak::clone(project),
                ))?;
            *should_break = true;
            let next_step = generate_next_step_non_inlined(
                block_info,
                blocks,
                context,
                final_next_blocks.clone(),
                flags,
            )?;
            generate_exhaustive_string_comparison(
                context.project()?.broadcasts().iter().cloned(),
                |broadcast| {
                    IrOpcode::event_broadcast_and_wait(EventBroadcastAndWaitFields {
                        broadcast,
                        poll_step,
                        next_step,
                    })
                },
                vec![IrOpcode::hq_yield(HqYieldFields {
                    mode: YieldMode::Schedule(next_step),
                })],
                context,
                project,
                flags,
            )?
        }
        BlockOpcode::sensing_askandwait => {
            let poll_step = context
                .project()?
                .new_owned_step(Step::new_poll_waiting_event(
                    context.clone(),
                    Weak::clone(project),
                ))?;
            *should_break = true;
            if context.target().is_stage() {
                let next_step = generate_next_step_non_inlined(
                    block_info,
                    blocks,
                    context,
                    final_next_blocks.clone(),
                    flags,
                )?;
                vec![IrOpcode::sensing_askandwait(SensingAskandwaitFields {
                    poll_step,
                    next_step,
                })]
            } else {
                let next_step = generate_next_step_inlined(
                    block_info,
                    blocks,
                    context,
                    final_next_blocks.clone(),
                    flags,
                )?;
                let real_next_step = Step::new(
                    None,
                    context.clone(),
                    vec![
                        IrOpcode::hq_text(HqTextFields("".into())),
                        IrOpcode::looks_say(LooksSayFields {
                            debug: false,
                            target_idx: context.target().index(),
                        }),
                        IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Inline(next_step),
                        }),
                    ],
                    Weak::clone(project),
                    true,
                )
                .clone_to_non_inlined(project)?;
                vec![
                    IrOpcode::looks_say(LooksSayFields {
                        debug: false,
                        target_idx: context.target().index(),
                    }),
                    IrOpcode::hq_text(HqTextFields("".into())),
                    IrOpcode::sensing_askandwait(SensingAskandwaitFields {
                        poll_step,
                        next_step: real_next_step,
                    }),
                ]
            }
        }
        BlockOpcode::control_wait => {
            let poll_step = context
                .project()?
                .new_owned_step(Step::new_poll_timer(context.clone(), Weak::clone(project)))?;
            *should_break = true;
            let next_step = generate_next_step_non_inlined(
                block_info,
                blocks,
                context,
                final_next_blocks.clone(),
                flags,
            )?;
            vec![IrOpcode::control_wait(ControlWaitFields {
                poll_step,
                next_step,
            })]
        }
        BlockOpcode::data_setvariableto => {
            let Sb3Field::ValueId(_val, maybe_id) =
                block_info.fields.get("VARIABLE").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field VARIABLE")
                })?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for VARIABLE field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for VARIABLE field")
            })?;
            let target = context.target();
            let variable = if let Some(var) = target.variables().get(&id) {
                var.clone()
            } else if let Some(var) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_variables()
                .get(&id)
            {
                var.clone()
            } else {
                hq_bad_proj!("variable not found")
            };
            *variable.is_used.try_borrow_mut()? = true;
            vec![IrOpcode::data_setvariableto(DataSetvariabletoFields {
                var: RefCell::new(variable.var.clone()),
                local_write: RefCell::new(false),
                first_write: RefCell::new(false),
            })]
        }
        BlockOpcode::data_changevariableby => {
            let Sb3Field::ValueId(_val, maybe_id) =
                block_info.fields.get("VARIABLE").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field VARIABLE")
                })?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for VARIABLE field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for VARIABLE field")
            })?;
            let target = context.target();
            let variable = if let Some(var) = target.variables().get(&id) {
                var.clone()
            } else if let Some(var) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_variables()
                .get(&id)
            {
                var.clone()
            } else {
                hq_bad_proj!("variable not found")
            };
            *variable.is_used.try_borrow_mut()? = true;
            vec![
                IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(variable.var.clone()),
                    local_read: RefCell::new(false),
                }),
                IrOpcode::operator_add,
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(variable.var.clone()),
                    local_write: RefCell::new(false),
                    first_write: RefCell::new(false),
                }),
            ]
        }
        BlockOpcode::data_variable => {
            let Sb3Field::ValueId(_val, maybe_id) =
                block_info.fields.get("VARIABLE").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field VARIABLE")
                })?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for VARIABLE field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for VARIABLE field")
            })?;
            let target = context.target();
            let variable = if let Some(var) = target.variables().get(&id) {
                var.clone()
            } else if let Some(var) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_variables()
                .get(&id)
            {
                var.clone()
            } else {
                hq_bad_proj!("variable not found")
            };
            *variable.is_used.try_borrow_mut()? = true;
            vec![IrOpcode::data_variable(DataVariableFields {
                var: RefCell::new(variable.var.clone()),
                local_read: RefCell::new(false),
            })]
        }
        BlockOpcode::data_showvariable => {
            let Sb3Field::ValueId(_val, maybe_id) =
                block_info.fields.get("VARIABLE").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field VARIABLE")
                })?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for VARIABLE field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for VARIABLE field")
            })?;
            let target = context.target();
            let variable = if let Some(var) = target.variables().get(&id) {
                var.clone()
            } else if let Some(var) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_variables()
                .get(&id)
            {
                var.clone()
            } else {
                hq_bad_proj!("variable not found")
            };
            *variable.is_used.try_borrow_mut()? = true;
            let Some(monitor) = variable.var.monitor().as_ref() else {
                hq_bad_proj!("tried to change visibility of variable without monitor");
            };
            *monitor.is_ever_visible.try_borrow_mut()? = true;
            vec![IrOpcode::data_visvariable(DataVisvariableFields {
                var: RefCell::new(variable.var.clone()),
                visible: true,
            })]
        }
        BlockOpcode::data_hidevariable => {
            let Sb3Field::ValueId(_val, maybe_id) =
                block_info.fields.get("VARIABLE").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field VARIABLE")
                })?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for VARIABLE field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for VARIABLE field")
            })?;
            let target = context.target();
            let variable = if let Some(var) = target.variables().get(&id) {
                var.clone()
            } else if let Some(var) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_variables()
                .get(&id)
            {
                var.clone()
            } else {
                hq_bad_proj!("variable not found")
            };
            *variable.is_used.try_borrow_mut()? = true;
            hq_assert!(
                variable.var.monitor().is_some(),
                "tried to change visibility of variable without monitor"
            );
            vec![IrOpcode::data_visvariable(DataVisvariableFields {
                var: RefCell::new(variable.var.clone()),
                visible: false,
            })]
        }
        BlockOpcode::data_deletealloflist => {
            let Sb3Field::ValueId(_val, maybe_id) = block_info
                .fields
                .get("LIST")
                .ok_or_else(|| make_hq_bad_proj!("invalid project.json - missing field LIST"))?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for LIST field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for LIST field")
            })?;
            let target = context.target();
            let list = if let Some(list) = target.lists().get(&id) {
                list.clone()
            } else if let Some(list) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_lists()
                .get(&id)
            {
                list.clone()
            } else {
                hq_bad_proj!("list not found")
            };
            *list.is_used.try_borrow_mut()? = true;
            *list.list.length_mutable().try_borrow_mut()? = true;
            vec![IrOpcode::data_deletealloflist(DataDeletealloflistFields {
                list: list.list.clone(),
            })]
        }
        BlockOpcode::data_addtolist => {
            let Sb3Field::ValueId(_val, maybe_id) = block_info
                .fields
                .get("LIST")
                .ok_or_else(|| make_hq_bad_proj!("invalid project.json - missing field LIST"))?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for LIST field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for LIST field")
            })?;
            let target = context.target();
            let list = if let Some(list) = target.lists().get(&id) {
                list.clone()
            } else if let Some(list) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_lists()
                .get(&id)
            {
                list.clone()
            } else {
                hq_bad_proj!("list not found")
            };
            *list.is_used.try_borrow_mut()? = true;
            *list.list.length_mutable().try_borrow_mut()? = true;
            vec![IrOpcode::data_addtolist(DataAddtolistFields {
                list: list.list.clone(),
            })]
        }
        BlockOpcode::data_itemnumoflist => {
            let Sb3Field::ValueId(_val, maybe_id) = block_info
                .fields
                .get("LIST")
                .ok_or_else(|| make_hq_bad_proj!("invalid project.json - missing field LIST"))?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for LIST field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for LIST field")
            })?;
            let target = context.target();
            let list = if let Some(list) = target.lists().get(&id) {
                list.clone()
            } else if let Some(list) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_lists()
                .get(&id)
            {
                list.clone()
            } else {
                hq_bad_proj!("list not found")
            };
            *list.is_used.try_borrow_mut()? = true;
            let item = RcVar::new_empty();
            let ret = RcVar::new(
                IrType::IntPos.or(IrType::IntZero),
                &VarVal::Int(0),
                None,
                flags,
            )?;
            let i = RcVar::new(
                IrType::IntPos.or(IrType::IntZero),
                &VarVal::Int(0),
                None,
                flags,
            )?;
            let condition = Rc::new(RefCell::new(Step::new(
                None,
                context.clone(),
                vec![
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(ret.clone()),
                        local_read: RefCell::new(true),
                    }),
                    IrOpcode::hq_integer(HqIntegerFields(0)),
                    IrOpcode::operator_equals,
                    IrOpcode::operator_not,
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(i.clone()),
                        local_read: RefCell::new(true),
                    }),
                    IrOpcode::hq_integer(HqIntegerFields(1)),
                    IrOpcode::operator_add,
                    IrOpcode::data_teevariable(DataTeevariableFields {
                        var: RefCell::new(i.clone()),
                        local_read_write: RefCell::new(true),
                    }),
                    IrOpcode::data_lengthoflist(DataLengthoflistFields {
                        list: list.list.clone(),
                    }),
                    IrOpcode::operator_gt,
                    IrOpcode::operator_or,
                ],
                Weak::clone(project),
                false,
            )));
            let body = Rc::new(RefCell::new(Step::new(
                None,
                context.clone(),
                vec![
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(i.clone()),
                        local_read: RefCell::new(true),
                    }),
                    IrOpcode::data_itemoflist(DataItemoflistFields {
                        list: list.list.clone(),
                    }),
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(item.clone()),
                        local_read: RefCell::new(true),
                    }),
                    IrOpcode::operator_equals,
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(i.clone()),
                        local_read: RefCell::new(true),
                    }),
                    IrOpcode::operator_multiply,
                    IrOpcode::data_setvariableto(DataSetvariabletoFields {
                        var: RefCell::new(ret.clone()),
                        local_write: RefCell::new(true),
                        first_write: RefCell::new(false),
                    }),
                ],
                Weak::clone(project),
                false,
            )));
            vec![
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(item),
                    local_write: RefCell::new(true),
                    first_write: RefCell::new(true),
                }),
                IrOpcode::hq_integer(HqIntegerFields(0)),
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(i),
                    local_write: RefCell::new(true),
                    first_write: RefCell::new(true),
                }),
                IrOpcode::hq_integer(HqIntegerFields(0)),
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(ret.clone()),
                    local_write: RefCell::new(true),
                    first_write: RefCell::new(true),
                }),
                IrOpcode::control_loop(ControlLoopFields {
                    first_condition: None,
                    condition,
                    body,
                    pre_body: None,
                    flip_if: true,
                }),
                IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(ret),
                    local_read: RefCell::new(true),
                }),
            ]
        }
        BlockOpcode::data_listcontainsitem => {
            let Sb3Field::ValueId(_val, maybe_id) = block_info
                .fields
                .get("LIST")
                .ok_or_else(|| make_hq_bad_proj!("invalid project.json - missing field LIST"))?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for LIST field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for LIST field")
            })?;
            let target = context.target();
            let list = if let Some(list) = target.lists().get(&id) {
                list.clone()
            } else if let Some(list) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_lists()
                .get(&id)
            {
                list.clone()
            } else {
                hq_bad_proj!("list not found")
            };
            *list.is_used.try_borrow_mut()? = true;
            let item = RcVar::new_empty();
            let ret = RcVar::new(IrType::Boolean, &VarVal::Bool(false), None, flags)?;
            let i = RcVar::new(
                IrType::IntPos.or(IrType::IntZero),
                &VarVal::Int(0),
                None,
                flags,
            )?;
            let condition = Rc::new(RefCell::new(Step::new(
                None,
                context.clone(),
                vec![
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(ret.clone()),
                        local_read: RefCell::new(true),
                    }),
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(i.clone()),
                        local_read: RefCell::new(true),
                    }),
                    IrOpcode::hq_integer(HqIntegerFields(1)),
                    IrOpcode::operator_add,
                    IrOpcode::data_teevariable(DataTeevariableFields {
                        var: RefCell::new(i.clone()),
                        local_read_write: RefCell::new(true),
                    }),
                    IrOpcode::data_lengthoflist(DataLengthoflistFields {
                        list: list.list.clone(),
                    }),
                    IrOpcode::operator_gt,
                    IrOpcode::operator_or,
                ],
                Weak::clone(project),
                false,
            )));
            let body = Rc::new(RefCell::new(Step::new(
                None,
                context.clone(),
                vec![
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(i.clone()),
                        local_read: RefCell::new(true),
                    }),
                    IrOpcode::data_itemoflist(DataItemoflistFields {
                        list: list.list.clone(),
                    }),
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(item.clone()),
                        local_read: RefCell::new(true),
                    }),
                    IrOpcode::operator_equals,
                    IrOpcode::data_setvariableto(DataSetvariabletoFields {
                        var: RefCell::new(ret.clone()),
                        local_write: RefCell::new(true),
                        first_write: RefCell::new(false),
                    }),
                ],
                Weak::clone(project),
                false,
            )));
            vec![
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(item),
                    local_write: RefCell::new(true),
                    first_write: RefCell::new(true),
                }),
                IrOpcode::hq_integer(HqIntegerFields(0)),
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(i),
                    local_write: RefCell::new(true),
                    first_write: RefCell::new(true),
                }),
                IrOpcode::hq_boolean(HqBooleanFields(false)),
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(ret.clone()),
                    local_write: RefCell::new(true),
                    first_write: RefCell::new(true),
                }),
                IrOpcode::control_loop(ControlLoopFields {
                    first_condition: None,
                    condition,
                    body,
                    pre_body: None,
                    flip_if: true,
                }),
                IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(ret),
                    local_read: RefCell::new(true),
                }),
            ]
        }
        BlockOpcode::data_insertatlist => {
            let Sb3Field::ValueId(_val, maybe_id) = block_info
                .fields
                .get("LIST")
                .ok_or_else(|| make_hq_bad_proj!("invalid project.json - missing field LIST"))?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for LIST field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for LIST field")
            })?;
            let target = context.target();
            let list = if let Some(list) = target.lists().get(&id) {
                list.clone()
            } else if let Some(list) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_lists()
                .get(&id)
            {
                list.clone()
            } else {
                hq_bad_proj!("list not found")
            };
            *list.is_used.try_borrow_mut()? = true;
            *list.list.length_mutable().try_borrow_mut()? = true;

            generate_list_index_op(
                &list.list,
                || {
                    IrOpcode::data_insertatlist(DataInsertatlistFields {
                        list: list.list.clone(),
                    })
                },
                None,
                true,
                true,
                None,
                context,
                project,
                flags,
            )?
        }
        BlockOpcode::data_deleteoflist => {
            let Sb3Field::ValueId(_val, maybe_id) = block_info
                .fields
                .get("LIST")
                .ok_or_else(|| make_hq_bad_proj!("invalid project.json - missing field LIST"))?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for LIST field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for LIST field")
            })?;
            let target = context.target();
            let list = if let Some(list) = target.lists().get(&id) {
                list.clone()
            } else if let Some(list) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_lists()
                .get(&id)
            {
                list.clone()
            } else {
                hq_bad_proj!("list not found")
            };
            *list.is_used.try_borrow_mut()? = true;
            *list.list.length_mutable().try_borrow_mut()? = true;

            generate_list_index_op(
                &list.list,
                || {
                    IrOpcode::data_deleteoflist(DataDeleteoflistFields {
                        list: list.list.clone(),
                    })
                },
                None,
                false,
                false,
                None,
                context,
                project,
                flags,
            )?
        }
        BlockOpcode::data_itemoflist => {
            let Sb3Field::ValueId(_val, maybe_id) = block_info
                .fields
                .get("LIST")
                .ok_or_else(|| make_hq_bad_proj!("invalid project.json - missing field LIST"))?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for LIST field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for LIST field")
            })?;
            let target = context.target();
            let list = if let Some(list) = target.lists().get(&id) {
                list.clone()
            } else if let Some(list) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_lists()
                .get(&id)
            {
                list.clone()
            } else {
                hq_bad_proj!("list not found")
            };
            *list.is_used.try_borrow_mut()? = true;

            generate_list_index_op(
                &list.list,
                || {
                    IrOpcode::data_itemoflist(DataItemoflistFields {
                        list: list.list.clone(),
                    })
                },
                None,
                false,
                false,
                Some(&IrOpcode::hq_text(HqTextFields("".into()))),
                context,
                project,
                flags,
            )?
        }
        BlockOpcode::data_lengthoflist => {
            let Sb3Field::ValueId(_val, maybe_id) = block_info
                .fields
                .get("LIST")
                .ok_or_else(|| make_hq_bad_proj!("invalid project.json - missing field LIST"))?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for LIST field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for LIST field")
            })?;
            let target = context.target();
            let list = if let Some(list) = target.lists().get(&id) {
                list.clone()
            } else if let Some(list) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_lists()
                .get(&id)
            {
                list.clone()
            } else {
                hq_bad_proj!("list not found")
            };
            *list.is_used.try_borrow_mut()? = true;
            vec![IrOpcode::data_lengthoflist(DataLengthoflistFields {
                list: list.list.clone(),
            })]
        }
        BlockOpcode::data_replaceitemoflist => {
            let Sb3Field::ValueId(_val, maybe_id) = block_info
                .fields
                .get("LIST")
                .ok_or_else(|| make_hq_bad_proj!("invalid project.json - missing field LIST"))?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for LIST field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for LIST field")
            })?;
            let target = context.target();
            let list = if let Some(list) = target.lists().get(&id) {
                list.clone()
            } else if let Some(list) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_lists()
                .get(&id)
            {
                list.clone()
            } else {
                hq_bad_proj!("list not found")
            };
            *list.is_used.try_borrow_mut()? = true;

            generate_list_index_op(
                &list.list,
                || {
                    IrOpcode::data_replaceitemoflist(DataReplaceitemoflistFields {
                        list: list.list.clone(),
                    })
                },
                None,
                true,
                false,
                None,
                context,
                project,
                flags,
            )?
        }
        BlockOpcode::data_listcontents => {
            let Sb3Field::ValueId(_val, maybe_id) = block_info
                .fields
                .get("LIST")
                .ok_or_else(|| make_hq_bad_proj!("invalid project.json - missing field LIST"))?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for LIST field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for LIST field")
            })?;
            let target = context.target();
            let list = if let Some(list) = target.lists().get(&id) {
                list.clone()
            } else if let Some(list) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_lists()
                .get(&id)
            {
                list.clone()
            } else {
                hq_bad_proj!("list not found")
            };
            *list.is_used.try_borrow_mut()? = true;
            vec![IrOpcode::data_listcontents(DataListcontentsFields {
                list: list.list.clone(),
            })]
        }
        BlockOpcode::control_stop => {
            let (Sb3Field::Value((Some(val),)) | Sb3Field::ValueId(Some(val), _)) =
                block_info.fields.get("STOP_OPTION").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field STOP_OPTION")
                })?
            else {
                hq_bad_proj!("invalid project.json - missing value for STOP_OPTION field")
            };
            let VarVal::String(operator) = val else {
                hq_bad_proj!("invalid project.json - non-string value for STOP_OPTION field")
            };
            match operator.to_lowercase().as_str() {
                "all" => vec![IrOpcode::control_stop_all],
                "this script" => vec![IrOpcode::hq_yield(HqYieldFields {
                    mode: if context.warp {
                        YieldMode::Return
                    } else {
                        YieldMode::None
                    },
                })],
                "other scripts in sprite" => {
                    hq_todo!("control_stop other scripts in sprite")
                }
                other => hq_bad_proj!("unknown mathop {}", other),
            }
        }

        BlockOpcode::control_if => 'block: {
            let BlockArrayOrId::Id(substack_id) = match block_info.inputs.get("SUBSTACK") {
                Some(input) => input,
                None => break 'block vec![IrOpcode::hq_drop],
            }
            .get_1()
            .ok_or_else(|| make_hq_bug!(""))?
            .clone()
            .ok_or_else(|| make_hq_bug!(""))?
            else {
                hq_bad_proj!("malformed SUBSTACK input")
            };
            let Some(substack_block) = blocks.get(&substack_id) else {
                hq_bad_proj!("SUBSTACK block doesn't seem to exist")
            };
            generate_if_else(
                (substack_block, substack_id),
                None,
                block_info,
                final_next_blocks,
                blocks,
                context,
                should_break,
                flags,
            )?
        }
        BlockOpcode::control_if_else => 'block: {
            let BlockArrayOrId::Id(substack1_id) = match block_info.inputs.get("SUBSTACK") {
                Some(input) => input,
                None => break 'block vec![IrOpcode::hq_drop],
            }
            .get_1()
            .ok_or_else(|| make_hq_bug!(""))?
            .clone()
            .ok_or_else(|| make_hq_bug!(""))?
            else {
                hq_bad_proj!("malformed SUBSTACK input")
            };
            let Some(substack1_block) = blocks.get(&substack1_id) else {
                hq_bad_proj!("SUBSTACK block doesn't seem to exist")
            };
            let BlockArrayOrId::Id(substack2_id) = match block_info.inputs.get("SUBSTACK2") {
                Some(input) => input,
                None => break 'block vec![IrOpcode::hq_drop],
            }
            .get_1()
            .ok_or_else(|| make_hq_bug!(""))?
            .clone()
            .ok_or_else(|| make_hq_bug!(""))?
            else {
                hq_bad_proj!("malformed SUBSTACK2 input")
            };
            let Some(substack2_block) = blocks.get(&substack2_id) else {
                hq_bad_proj!("SUBSTACK2 block doesn't seem to exist")
            };
            generate_if_else(
                (substack1_block, substack1_id),
                Some((substack2_block, substack2_id)),
                block_info,
                final_next_blocks,
                blocks,
                context,
                should_break,
                flags,
            )?
        }
        BlockOpcode::control_forever => {
            let condition_instructions = vec![IrOpcode::hq_boolean(HqBooleanFields(true))];
            let first_condition_instructions = None;
            generate_loop(
                context.warp,
                should_break,
                block_info,
                blocks,
                context,
                final_next_blocks.clone(),
                first_condition_instructions,
                condition_instructions,
                None,
                false,
                vec![],
                flags,
            )?
        }
        BlockOpcode::control_repeat => {
            let variable = RcVar::new(IrType::Int, &VarVal::Int(0), None, flags)?;
            let local = context.warp;
            let condition_instructions = vec![
                IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(variable.clone()),
                    local_read: RefCell::new(local),
                }),
                IrOpcode::hq_integer(HqIntegerFields(1)),
                IrOpcode::operator_subtract,
                IrOpcode::data_teevariable(DataTeevariableFields {
                    var: RefCell::new(variable.clone()),
                    local_read_write: RefCell::new(local),
                }),
            ];
            let first_condition_instructions =
                Some(vec![IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(variable.clone()),
                    local_read: RefCell::new(local),
                })]);
            let setup_instructions = vec![
                IrOpcode::hq_cast(HqCastFields(IrType::Int)),
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(variable),
                    local_write: RefCell::new(local),
                    first_write: RefCell::new(local),
                }),
            ];
            generate_loop(
                context.warp,
                should_break,
                block_info,
                blocks,
                context,
                final_next_blocks.clone(),
                first_condition_instructions,
                condition_instructions,
                None,
                false,
                setup_instructions,
                flags,
            )?
        }
        BlockOpcode::control_for_each => {
            let Sb3Field::ValueId(_val, maybe_id) =
                block_info.fields.get("VARIABLE").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field VARIABLE")
                })?
            else {
                hq_bad_proj!("invalid project.json - missing variable id for VARIABLE field");
            };
            let id = maybe_id.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null variable id for VARIABLE field")
            })?;
            let target = context.target();
            let variable = if let Some(var) = target.variables().get(&id) {
                var.clone()
            } else if let Some(var) = context
                .target()
                .project()
                .upgrade()
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Project>"))?
                .global_variables()
                .get(&id)
            {
                var.clone()
            } else {
                hq_bad_proj!("variable not found")
            };
            *variable.is_used.try_borrow_mut()? = true;
            let counter = RcVar::new(IrType::Int, &VarVal::Int(0), None, flags)?;
            let local = context.warp;
            let condition_instructions = vec![
                IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(counter.clone()),
                    local_read: RefCell::new(local),
                }),
                IrOpcode::hq_integer(HqIntegerFields(1)),
                IrOpcode::operator_add,
                IrOpcode::data_teevariable(DataTeevariableFields {
                    var: RefCell::new(counter.clone()),
                    local_read_write: RefCell::new(local),
                }),
            ]
            .into_iter()
            .chain(inputs(
                block_info,
                blocks,
                context,
                &context.target().project(),
                flags,
            )?)
            .chain(vec![IrOpcode::operator_lt])
            .collect();
            let first_condition_instructions = Some(
                vec![IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(counter.clone()),
                    local_read: RefCell::new(local),
                })]
                .into_iter()
                .chain(inputs(
                    block_info,
                    blocks,
                    context,
                    &context.target().project(),
                    flags,
                )?)
                .chain(vec![IrOpcode::operator_lt])
                .collect(),
            );
            let setup_instructions = vec![
                IrOpcode::hq_drop,
                IrOpcode::hq_integer(HqIntegerFields(0)),
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(counter.clone()),
                    local_write: RefCell::new(local),
                    first_write: RefCell::new(true),
                }),
            ];
            let pre_body_instructions = Some(vec![
                IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(counter),
                    local_read: RefCell::new(local),
                }),
                IrOpcode::hq_integer(HqIntegerFields(1)),
                IrOpcode::operator_add,
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(variable.var.clone()),
                    local_write: RefCell::new(false),
                    first_write: RefCell::new(false),
                }),
            ]);
            generate_loop(
                context.warp,
                should_break,
                block_info,
                blocks,
                context,
                final_next_blocks.clone(),
                first_condition_instructions,
                condition_instructions,
                pre_body_instructions,
                false,
                setup_instructions,
                flags,
            )?
        }
        BlockOpcode::control_repeat_until | BlockOpcode::control_wait_until => {
            let condition_instructions = inputs(
                block_info,
                blocks,
                context,
                &context.target().project(),
                flags,
            )?;
            let first_condition_instructions = None;
            let setup_instructions = vec![IrOpcode::hq_drop];
            generate_loop(
                context.warp,
                should_break,
                block_info,
                blocks,
                context,
                final_next_blocks.clone(),
                first_condition_instructions,
                condition_instructions,
                None,
                true,
                setup_instructions,
                flags,
            )?
        }
        BlockOpcode::control_while => {
            let condition_instructions = inputs(
                block_info,
                blocks,
                context,
                &context.target().project(),
                flags,
            )?;
            let first_condition_instructions = None;
            let setup_instructions = vec![IrOpcode::hq_drop];
            generate_loop(
                context.warp,
                should_break,
                block_info,
                blocks,
                context,
                final_next_blocks.clone(),
                first_condition_instructions,
                condition_instructions,
                None,
                false,
                setup_instructions,
                flags,
            )?
        }
        BlockOpcode::procedures_call => 'proc_block: {
            let target = context.target();
            let procs = target.procedures()?;
            let serde_json::Value::String(proccode) = block_info
                .mutation
                .mutations
                .get("proccode")
                .ok_or_else(|| make_hq_bad_proj!("missing proccode on procedures_call"))?
            else {
                hq_bad_proj!("non-string proccode on procedures_call")
            };
            let Some(proc) = procs.get(proccode.as_str()) else {
                break 'proc_block vec![];
            };
            let warp = context.warp || proc.always_warped();
            if warp {
                proc.compile_warped(blocks, flags)?;
                vec![IrOpcode::procedures_call_warp(ProceduresCallWarpFields {
                    proc: Rc::clone(proc),
                })]
            } else {
                *should_break = true;
                let next_step = generate_next_step_non_inlined(
                    block_info,
                    blocks,
                    context,
                    final_next_blocks.clone(),
                    flags,
                )?;
                proc.compile_nonwarped(blocks, flags)?;
                vec![IrOpcode::procedures_call_nonwarp(
                    ProceduresCallNonwarpFields {
                        proc: Rc::clone(proc),
                        next_step,
                    },
                )]
            }
        }
        BlockOpcode::argument_reporter_boolean => {
            procedure_argument(ProcArgType::Boolean, block_info, context)?
        }
        BlockOpcode::argument_reporter_string_number => {
            procedure_argument(ProcArgType::StringNumber, block_info, context)?
        }
        BlockOpcode::looks_show => vec![
            IrOpcode::hq_boolean(HqBooleanFields(true)),
            IrOpcode::looks_setvisible,
        ],
        BlockOpcode::looks_hide => vec![
            IrOpcode::hq_boolean(HqBooleanFields(false)),
            IrOpcode::looks_setvisible,
        ],
        BlockOpcode::pen_clear => vec![IrOpcode::pen_clear],
        BlockOpcode::pen_penDown => vec![IrOpcode::pen_pendown],
        BlockOpcode::pen_penUp => vec![IrOpcode::pen_penup],
        BlockOpcode::pen_setPenSizeTo => vec![IrOpcode::pen_setpensizeto],
        BlockOpcode::pen_setPenColorToColor => {
            vec![IrOpcode::pen_setpencolortocolor]
        }
        BlockOpcode::pen_changePenColorParamBy => {
            vec![IrOpcode::pen_changecolorparamby]
        }
        BlockOpcode::pen_setPenColorParamTo => {
            vec![IrOpcode::pen_setpencolorparamto]
        }
        BlockOpcode::pen_menu_colorParam => {
            let maybe_val = match block_info.fields.get("colorParam").ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - missing field colorParam")
            })? {
                Sb3Field::Value((v,)) | Sb3Field::ValueId(v, _) => v,
            };
            let val_varval = maybe_val.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null value for OPERATOR field")
            })?;
            let VarVal::String(val) = val_varval else {
                hq_bad_proj!("invalid project.json - expected colorParam field to be string");
            };
            vec![IrOpcode::hq_text(HqTextFields(val))]
        }
        BlockOpcode::looks_setsizeto => vec![IrOpcode::looks_setsizeto],
        BlockOpcode::looks_size => vec![IrOpcode::looks_size],
        BlockOpcode::looks_changesizeby => vec![
            IrOpcode::looks_size,
            IrOpcode::operator_add,
            IrOpcode::looks_setsizeto,
        ],
        BlockOpcode::looks_switchcostumeto => vec![IrOpcode::looks_switchcostumeto],
        BlockOpcode::looks_switchbackdropto => {
            vec![IrOpcode::looks_switchbackdropto]
        }
        BlockOpcode::looks_costumenumbername => {
            let (Sb3Field::Value((val,)) | Sb3Field::ValueId(val, _)) =
                block_info.fields.get("NUMBER_NAME").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field NUMBER_NAME")
                })?;
            let VarVal::String(number_name) = val.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null costume name for NUMBER_NAME field")
            })?
            else {
                hq_bad_proj!("invalid project.json - NUMBER_NAME field is not of type String");
            };
            match &*number_name {
                "number" => vec![IrOpcode::looks_costumenumber],
                "name" => vec![IrOpcode::looks_costumename],
                _ => hq_bad_proj!("invalid value for NUMBER_NAME field"),
            }
        }
        BlockOpcode::looks_backdropnumbername => {
            let (Sb3Field::Value((val,)) | Sb3Field::ValueId(val, _)) =
                block_info.fields.get("NUMBER_NAME").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field NUMBER_NAME")
                })?;
            let VarVal::String(number_name) = val.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null backdrop name for NUMBER_NAME field")
            })?
            else {
                hq_bad_proj!("invalid project.json - NUMBER_NAME field is not of type String");
            };
            match &*number_name {
                "number" => vec![IrOpcode::looks_backdropnumber],
                "name" => hq_todo!("backdrop name"),
                _ => hq_bad_proj!("invalid value for NUMBER_NAME field"),
            }
        }
        BlockOpcode::looks_backdrops => {
            let (Sb3Field::Value((val,)) | Sb3Field::ValueId(val, _)) =
                block_info.fields.get("BACKDROP").ok_or_else(|| {
                    make_hq_bad_proj!("invalid project.json - missing field BACKDROP")
                })?;
            let VarVal::String(backdrop_name) = val.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null backdrop name for BACKROP field")
            })?
            else {
                hq_bad_proj!("invalid project.json - BACKDROP field is not of type String");
            };
            let backdrop_index: i32 = context
                .project()?
                .backdrops()
                .iter()
                .find_position(|costume| costume.name == backdrop_name)
                .ok_or_else(|| make_hq_bug!("backdrop index not found"))?
                .0
                .try_into()
                .map_err(|_| make_hq_bug!("backdrop index out of bounds"))?;
            vec![IrOpcode::hq_integer(HqIntegerFields(backdrop_index))]
        }
        BlockOpcode::looks_nextcostume => {
            vec![
                IrOpcode::looks_costumenumber,
                IrOpcode::hq_integer(HqIntegerFields(1)),
                IrOpcode::operator_add,
                IrOpcode::hq_integer(HqIntegerFields(
                    context
                        .target()
                        .costumes()
                        .len()
                        .try_into()
                        .map_err(|_| make_hq_bug!("costumes length out of bounds"))?,
                )),
                IrOpcode::operator_modulo,
                IrOpcode::looks_switchcostumeto,
            ]
        }
        BlockOpcode::looks_nextbackdrop => {
            vec![
                IrOpcode::looks_backdropnumber,
                IrOpcode::hq_integer(HqIntegerFields(1)),
                IrOpcode::operator_add,
                IrOpcode::hq_integer(HqIntegerFields(
                    context
                        .project()?
                        .backdrops()
                        .len()
                        .try_into()
                        .map_err(|_| make_hq_bug!("backdrops length out of bounds"))?,
                )),
                IrOpcode::operator_modulo,
                IrOpcode::looks_switchbackdropto,
            ]
        }
        BlockOpcode::looks_costume => {
            let (Sb3Field::Value((val,)) | Sb3Field::ValueId(val, _)) = block_info
                .fields
                .get("COSTUME")
                .ok_or_else(|| make_hq_bad_proj!("invalid project.json - missing field COSTUME"))?;
            let VarVal::String(name) = val.clone().ok_or_else(|| {
                make_hq_bad_proj!("invalid project.json - null costume name for COSTUME field")
            })?
            else {
                hq_bad_proj!("invalid project.json - COSTUME field is not of type String");
            };
            let index = context
                .target()
                .costumes()
                .iter()
                .position(|costume| costume.name == name)
                .ok_or_else(|| make_hq_bad_proj!("missing costume with name {}", name))?;
            vec![IrOpcode::hq_integer(HqIntegerFields(
                index
                    .try_into()
                    .map_err(|_| make_hq_bug!("costume index out of bounds"))?,
            ))]
        }
        other => hq_todo!("unimplemented block: {:?}", other),
    })
}
