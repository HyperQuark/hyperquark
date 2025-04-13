use super::{IrProject, RcVar, Step, Type as IrType};
use crate::instructions::{fields::*, IrOpcode, YieldMode};
use crate::ir::Variable;
use crate::prelude::*;
use crate::sb3;
use sb3::{Block, BlockArray, BlockArrayOrId, BlockInfo, BlockMap, BlockOpcode, Input};

use super::context::StepContext;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StackMode {
    Stack,
    NotStack,
}

fn insert_casts(mut blocks: Vec<IrOpcode>) -> HQResult<Vec<IrOpcode>> {
    let mut type_stack: Vec<(IrType, usize)> = vec![]; // a vector of types, and where they came from
    let mut casts: Vec<(usize, IrType)> = vec![]; // a vector of cast targets, and where they're needed
    for (i, block) in blocks.iter().enumerate() {
        let expected_inputs = block.acceptable_inputs();
        if type_stack.len() < expected_inputs.len() {
            hq_bug!("didn't have enough inputs on the type stack")
        }
        let actual_inputs: Vec<_> = type_stack
            .splice((type_stack.len() - expected_inputs.len()).., [])
            .collect();
        for (&expected, actual) in core::iter::zip(expected_inputs.iter(), actual_inputs) {
            if !expected
                .base_types()
                .any(|ty1| actual.0.base_types().any(|ty2| ty2 == ty1))
            {
                casts.push((actual.1, expected));
            }
        }
        // TODO: make this more specific by using the actual input types post-cast
        if let Some(output) = block.output_type(expected_inputs)? {
            type_stack.push((output, i));
        }
    }
    for (pos, ty) in casts {
        blocks.insert(pos + 1, IrOpcode::hq_cast(HqCastFields(ty)));
    }
    Ok(blocks)
}

#[derive(Clone, Debug, PartialEq)]
pub struct NextBlockInfo {
    pub yield_first: bool,
    pub id: Box<str>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NextBlocks {
    /// if this is empty, terminate the thread afterwards
    NextBlocks(Vec<NextBlockInfo>),
    /// don't terminate the thread. do nothing. used in loop bodies
    /// or elsewhere where next-ness is handled elsewhere
    NothingAtAll,
}

impl NextBlocks {
    pub fn new() -> Self {
        NextBlocks::NextBlocks(vec![])
    }

    pub fn extend_with_inner(&self, new: NextBlockInfo) -> Self {
        match self {
            NextBlocks::NextBlocks(inner) => {
                let mut cloned = inner.clone();
                cloned.push(new);
                NextBlocks::NextBlocks(cloned)
            }
            NextBlocks::NothingAtAll => NextBlocks::NextBlocks(vec![new]),
        }
    }

    pub fn pop_inner(self) -> (Option<NextBlockInfo>, Self) {
        if let NextBlocks::NextBlocks(mut vec) = self {
            let popped = vec.pop();
            (popped, NextBlocks::NextBlocks(vec))
        } else {
            (None, self)
        }
    }
}

pub fn from_block(
    block: &Block,
    stack_mode: StackMode,
    blocks: &BlockMap,
    context: &StepContext,
    project: Weak<IrProject>,
    final_next_blocks: NextBlocks,
) -> HQResult<Vec<IrOpcode>> {
    insert_casts(match block {
        Block::Normal { block_info, .. } => from_normal_block(
            block_info,
            stack_mode,
            blocks,
            context,
            Weak::clone(&project),
            final_next_blocks,
        )?
        .to_vec(),
        Block::Special(block_array) => vec![from_special_block(block_array, context)?],
    })
}

pub fn input_names(block_info: &BlockInfo, context: &StepContext) -> HQResult<Vec<String>> {
    let opcode = &block_info.opcode;
    // target and procs need to be declared outside of the match block
    // to prevent lifetime issues
    let target = context
        .target
        .upgrade()
        .ok_or(make_hq_bug!("couldn't upgrade Weak<Target>"))?;
    let procs = target.procedures()?;
    Ok(match opcode {
        BlockOpcode::looks_say => vec!["MESSAGE"],
        BlockOpcode::operator_add
        | BlockOpcode::operator_divide
        | BlockOpcode::operator_subtract
        | BlockOpcode::operator_multiply => vec!["NUM1", "NUM2"],
        BlockOpcode::operator_lt | BlockOpcode::operator_gt => vec!["OPERAND1", "OPERAND2"],
        BlockOpcode::operator_join => vec!["STRING1", "STRING2"],
        BlockOpcode::sensing_dayssince2000
        | BlockOpcode::data_variable
        | BlockOpcode::argument_reporter_boolean
        | BlockOpcode::argument_reporter_string_number => vec![],
        BlockOpcode::data_setvariableto => vec!["VALUE"],
        BlockOpcode::control_if => vec!["CONDITION"],
        BlockOpcode::operator_not => vec!["OPERAND"],
        BlockOpcode::control_repeat => vec!["TIMES"],
        BlockOpcode::procedures_call => {
            let serde_json::Value::String(proccode) = block_info
                .mutation
                .mutations
                .get("proccode")
                .ok_or(make_hq_bad_proj!("missing proccode on procedures_call"))?
            else {
                hq_bad_proj!("non-string proccode on procedures_call");
            };
            let Some(proc) = procs.get(proccode.as_str()) else {
                hq_bad_proj!("procedures_call proccode doesn't exist")
            };
            proc.context().arg_ids().iter().map(|b| &**b).collect()
        }
        other => hq_todo!("unimplemented input_names for {:?}", other),
    }
    .into_iter()
    .map(String::from)
    .collect())
}

pub fn inputs(
    block_info: &BlockInfo,
    blocks: &BlockMap,
    context: &StepContext,
    project: Weak<IrProject>,
) -> HQResult<Vec<IrOpcode>> {
    Ok(input_names(block_info, context)?
        .into_iter()
        .map(|name| -> HQResult<Vec<IrOpcode>> {
            match match block_info.inputs.get((*name).into()) {
                Some(input) => input,
                None => {
                    if name.starts_with("CONDITION") {
                        &Input::NoShadow(
                            0,
                            Some(BlockArrayOrId::Array(BlockArray::NumberOrAngle(6, 0.0))),
                        )
                    } else {
                        hq_bad_proj!("missing input {}", name)
                    }
                }
            } {
                Input::NoShadow(_, Some(block)) | Input::Shadow(_, Some(block), _) => match block {
                    BlockArrayOrId::Array(arr) => Ok(vec![from_special_block(arr, context)?]),
                    BlockArrayOrId::Id(id) => from_block(
                        blocks
                            .get(id)
                            .ok_or(make_hq_bad_proj!("block for input {} doesn't exist", name))?,
                        StackMode::NotStack,
                        blocks,
                        context,
                        Weak::clone(&project),
                        NextBlocks::new(),
                    ),
                },
                _ => hq_bad_proj!("missing input block for {}", name),
            }
        })
        .collect::<HQResult<Vec<_>>>()?
        .iter()
        .flatten()
        .cloned()
        .collect())
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub enum ProcArgType {
    Boolean,
    StringNumber,
}

impl ProcArgType {
    fn default_block(&self) -> Vec<IrOpcode> {
        vec![match self {
            ProcArgType::Boolean => IrOpcode::hq_integer(HqIntegerFields(0)),
            ProcArgType::StringNumber => IrOpcode::hq_text(HqTextFields("".into())),
        }]
    }
}

fn procedure_argument(
    arg_type: ProcArgType,
    block_info: &BlockInfo,
    context: &StepContext,
) -> HQResult<Vec<IrOpcode>> {
    let Some(proc_context) = context.proc_context.clone() else {
        return Ok(arg_type.default_block());
    };
    let sb3::VarVal::String(arg_name) = block_info
        .fields
        .get("VALUE")
        .ok_or(make_hq_bad_proj!("missing VALUE field for proc argument"))?
        .get_0()
        .clone()
        .ok_or(make_hq_bad_proj!("missing value of VALUE field"))?
    else {
        hq_bad_proj!("non-string proc argument name")
    };
    let Some(index) = proc_context
        .arg_names()
        .iter()
        .position(|name| *name == arg_name)
    else {
        return Ok(arg_type.default_block());
    };
    let expected_type = proc_context
        .arg_types()
        .get(index)
        .ok_or(make_hq_bad_proj!(
            "argument index not in range of argumenttypes"
        ))?;
    hq_assert!(
        (arg_type == ProcArgType::Boolean && *expected_type == IrType::Boolean)
            || (arg_type == ProcArgType::StringNumber
                && (*expected_type == IrType::String || *expected_type == IrType::Number)),
        "argument block doesn't match actual argument type"
    );
    Ok(vec![IrOpcode::procedures_argument(
        ProceduresArgumentFields(index, *expected_type),
    )])
}

fn from_normal_block(
    block_info: &BlockInfo,
    stack_mode: StackMode,
    blocks: &BlockMap,
    context: &StepContext,
    project: Weak<IrProject>,
    final_next_blocks: NextBlocks,
) -> HQResult<Box<[IrOpcode]>> {
    let mut curr_block = Some(block_info);
    let mut opcodes = vec![];
    let mut should_break = false;
    while let Some(block_info) = curr_block {
        if stack_mode == StackMode::Stack {
            crate::log(format!("{:?}", block_info.opcode).as_str());
        }
        opcodes.append(
            &mut inputs(block_info, blocks, context, Weak::clone(&project))?
                .into_iter()
                .chain(match &block_info.opcode {
                    BlockOpcode::operator_add => vec![IrOpcode::operator_add],
                    BlockOpcode::operator_subtract => vec![IrOpcode::operator_subtract],
                    BlockOpcode::operator_multiply => vec![IrOpcode::operator_multiply],
                    BlockOpcode::operator_divide => vec![IrOpcode::operator_divide],
                    BlockOpcode::looks_say => vec![IrOpcode::looks_say],
                    BlockOpcode::operator_join => vec![IrOpcode::operator_join],
                    BlockOpcode::sensing_dayssince2000 => vec![IrOpcode::sensing_dayssince2000],
                    BlockOpcode::operator_lt => vec![IrOpcode::operator_lt],
                    BlockOpcode::operator_gt => vec![IrOpcode::operator_gt],
                    BlockOpcode::operator_not => vec![IrOpcode::operator_not],
                    BlockOpcode::data_setvariableto => {
                        let sb3::Field::ValueId(_val, maybe_id) =
                            block_info.fields.get("VARIABLE").ok_or(make_hq_bad_proj!(
                                "invalid project.json - missing field VARIABLE"
                            ))?
                        else {
                            hq_bad_proj!(
                                "invalid project.json - missing variable id for VARIABLE field"
                            );
                        };
                        let id = maybe_id.clone().ok_or(make_hq_bad_proj!(
                            "invalid project.json - null variable id for VARIABLE field"
                        ))?;
                        let target = context
                            .target
                            .upgrade()
                            .ok_or(make_hq_bug!("couldn't upgrade Weak"))?;
                        let variable = if let Some(var) = target.variables().get(&id) {
                            var
                        } else {
                            hq_todo!("global variables")
                        };
                        vec![IrOpcode::data_setvariableto(DataSetvariabletoFields(
                            RcVar(Rc::clone(variable)),
                        ))]
                    }
                    BlockOpcode::data_variable => {
                        let sb3::Field::ValueId(_val, maybe_id) =
                            block_info.fields.get("VARIABLE").ok_or(make_hq_bad_proj!(
                                "invalid project.json - missing field VARIABLE"
                            ))?
                        else {
                            hq_bad_proj!(
                                "invalid project.json - missing variable id for VARIABLE field"
                            );
                        };
                        let id = maybe_id.clone().ok_or(make_hq_bad_proj!(
                            "invalid project.json - null variable id for VARIABLE field"
                        ))?;
                        let target = context
                            .target
                            .upgrade()
                            .ok_or(make_hq_bug!("couldn't upgrade Weak"))?;
                        let variable = if let Some(var) = target.variables().get(&id) {
                            var
                        } else {
                            hq_todo!("global variables")
                        };
                        vec![IrOpcode::data_variable(DataVariableFields(RcVar(
                            Rc::clone(variable),
                        )))]
                    }
                    BlockOpcode::control_if => 'block: {
                        let BlockArrayOrId::Id(substack_id) =
                            match block_info.inputs.get("SUBSTACK") {
                                Some(input) => input,
                                None => break 'block vec![IrOpcode::hq_drop],
                            }
                            .get_1()
                            .ok_or(make_hq_bug!(""))?
                            .clone()
                            .ok_or(make_hq_bug!(""))?
                        else {
                            hq_bad_proj!("malformed SUBSTACK input")
                        };
                        let Some(substack_block) = blocks.get(&substack_id) else {
                            hq_bad_proj!("SUBSTACK block doesn't seem to exist")
                        };
                        let (next_block, if_next_blocks, else_next_blocks) =
                            if let Some(ref next_block) = block_info.next {
                                (
                                    Some(next_block.clone()),
                                    final_next_blocks.extend_with_inner(NextBlockInfo {
                                        yield_first: false,
                                        id: next_block.clone(),
                                    }),
                                    final_next_blocks.clone(),
                                )
                            } else if let (Some(next_block_info), popped_next_blocks) =
                                final_next_blocks.clone().pop_inner()
                            {
                                (
                                    Some(next_block_info.id),
                                    final_next_blocks.clone(),
                                    popped_next_blocks,
                                )
                            } else {
                                (None, NextBlocks::new(), NextBlocks::new())
                            };
                        let if_step = Step::from_block(
                            substack_block,
                            substack_id,
                            blocks,
                            context.clone(),
                            context.project()?,
                            if_next_blocks,
                        )?;
                        let else_step = if next_block.is_some() {
                            let Some(next_block_block) = blocks.get(&next_block.clone().unwrap())
                            else {
                                hq_bad_proj!("next block doesn't exist")
                            };
                            Step::from_block(
                                next_block_block,
                                next_block.clone().unwrap(),
                                blocks,
                                context.clone(),
                                context.project()?,
                                else_next_blocks,
                            )?
                        } else {
                            Step::new_terminating(context.clone(), context.project()?)?
                        };
                        should_break = true;
                        vec![IrOpcode::control_if_else(ControlIfElseFields {
                            branch_if: if_step,
                            branch_else: else_step,
                        })]
                    }
                    BlockOpcode::control_repeat => 'block: {
                        // todo: use control_loop for warped loops
                        let BlockArrayOrId::Id(substack_id) =
                            match block_info.inputs.get("SUBSTACK") {
                                Some(input) => input,
                                None => break 'block vec![IrOpcode::hq_drop],
                            }
                            .get_1()
                            .ok_or(make_hq_bug!(""))?
                            .clone()
                            .ok_or(make_hq_bug!(""))?
                        else {
                            hq_bad_proj!("malformed SUBSTACK input")
                        };
                        let Some(substack_block) = blocks.get(&substack_id) else {
                            hq_bad_proj!("SUBSTACK block doesn't seem to exist")
                        };
                        if !context.warp {
                            should_break = true;
                            let (next_block, outer_next_blocks) =
                                if let Some(ref next_block) = block_info.next {
                                    (Some(next_block.clone()), final_next_blocks.clone())
                                } else if let (Some(next_block_info), popped_next_blocks) =
                                    final_next_blocks.clone().pop_inner()
                                {
                                    (Some(next_block_info.id), popped_next_blocks)
                                } else {
                                    (None, NextBlocks::new())
                                };
                            let next_step = if next_block.is_some() {
                                let Some(next_block_block) =
                                    blocks.get(&next_block.clone().unwrap())
                                else {
                                    hq_bad_proj!("next block doesn't exist")
                                };
                                Step::from_block(
                                    next_block_block,
                                    next_block.clone().unwrap(),
                                    blocks,
                                    context.clone(),
                                    context.project()?,
                                    outer_next_blocks,
                                )?
                            } else {
                                Step::new_terminating(context.clone(), context.project()?)?
                            };
                            let variable =
                                RcVar(Rc::new(Variable::new(IrType::Int, sb3::VarVal::Float(0.0))));
                            let substack_blocks = from_block(
                                substack_block,
                                StackMode::Stack,
                                blocks,
                                context,
                                context.project()?,
                                NextBlocks::NothingAtAll,
                            )?;
                            let substack_step = Step::new_rc(
                                None,
                                context.clone(),
                                substack_blocks,
                                context.project()?,
                            )?;
                            let condition_step = Step::new_rc(
                                None,
                                context.clone(),
                                vec![
                                    IrOpcode::data_variable(DataVariableFields(variable.clone())),
                                    IrOpcode::hq_integer(HqIntegerFields(1)),
                                    IrOpcode::operator_subtract,
                                    IrOpcode::data_teevariable(DataTeevariableFields(
                                        variable.clone(),
                                    )),
                                    IrOpcode::hq_integer(HqIntegerFields(0)),
                                    IrOpcode::operator_gt,
                                    IrOpcode::control_if_else(ControlIfElseFields {
                                        branch_if: Rc::clone(&substack_step),
                                        branch_else: Rc::clone(&next_step),
                                    }),
                                ],
                                context.project()?,
                            )?;
                            substack_step
                                .opcodes_mut()?
                                .push(IrOpcode::hq_yield(HqYieldFields {
                                    mode: YieldMode::Schedule(Rc::downgrade(&condition_step)),
                                }));
                            vec![
                                IrOpcode::hq_cast(HqCastFields(IrType::Int)),
                                IrOpcode::data_teevariable(DataTeevariableFields(variable)),
                                IrOpcode::hq_integer(HqIntegerFields(0)),
                                IrOpcode::operator_gt,
                                IrOpcode::control_if_else(ControlIfElseFields {
                                    branch_if: substack_step,
                                    branch_else: next_step,
                                }),
                            ]
                        } else {
                            // TODO: this shoud use a local variable (as opposed to global)
                            let variable =
                                RcVar(Rc::new(Variable::new(IrType::Int, sb3::VarVal::Float(0.0))));
                            let substack_blocks = from_block(
                                substack_block,
                                StackMode::Stack,
                                blocks,
                                context,
                                context.project()?,
                                NextBlocks::NothingAtAll,
                            )?;
                            let substack_step = Step::new_rc(
                                None,
                                context.clone(),
                                substack_blocks,
                                context.project()?,
                            )?;
                            let condition_step = Step::new_rc(
                                None,
                                context.clone(),
                                vec![
                                    IrOpcode::data_variable(DataVariableFields(variable.clone())),
                                    IrOpcode::hq_integer(HqIntegerFields(1)),
                                    IrOpcode::operator_subtract,
                                    IrOpcode::data_teevariable(DataTeevariableFields(
                                        variable.clone(),
                                    )),
                                    IrOpcode::hq_integer(HqIntegerFields(0)),
                                    IrOpcode::operator_gt,
                                ],
                                context.project()?,
                            )?;
                            let first_condition_step = Step::new_rc(
                                None,
                                context.clone(),
                                vec![
                                    IrOpcode::data_variable(DataVariableFields(variable.clone())),
                                    IrOpcode::hq_integer(HqIntegerFields(0)),
                                    IrOpcode::operator_gt,
                                ],
                                context.project()?,
                            )?;
                            vec![
                                IrOpcode::hq_cast(HqCastFields(IrType::Int)),
                                IrOpcode::data_setvariableto(DataSetvariabletoFields(variable)),
                                IrOpcode::control_loop(ControlLoopFields {
                                    first_condition: Some(first_condition_step),
                                    condition: condition_step,
                                    body: substack_step,
                                }),
                            ]
                        }
                    }
                    BlockOpcode::procedures_call => {
                        let target = context
                            .target
                            .upgrade()
                            .ok_or(make_hq_bug!("couldn't upgrade Weak<Target>"))?;
                        let procs = target.procedures()?;
                        let serde_json::Value::String(proccode) = block_info
                            .mutation
                            .mutations
                            .get("proccode")
                            .ok_or(make_hq_bad_proj!("missing proccode on procedures_call"))?
                        else {
                            hq_bad_proj!("non-string proccode on procedures_call")
                        };
                        let proc = procs.get(proccode.as_str()).ok_or(make_hq_bad_proj!(
                            "non-existant proccode on procedures_call"
                        ))?;
                        let warp = context.warp || proc.always_warped();
                        if warp {
                            proc.compile_warped(blocks)?;
                            vec![IrOpcode::procedures_call_warp(ProceduresCallWarpFields {
                                proc: Rc::clone(proc),
                            })]
                        } else {
                            hq_todo!("non-warped procedures")
                        }
                    }
                    BlockOpcode::argument_reporter_boolean => {
                        procedure_argument(ProcArgType::Boolean, block_info, context)?
                    }
                    BlockOpcode::argument_reporter_string_number => {
                        procedure_argument(ProcArgType::StringNumber, block_info, context)?
                    }
                    other => hq_todo!("unimplemented block: {:?}", other),
                })
                .collect(),
        );
        if should_break {
            break;
        }
        if stack_mode == StackMode::Stack {
            curr_block = if let Some(ref next_id) = block_info.next {
                let next_block = blocks
                    .get(next_id)
                    .ok_or(make_hq_bad_proj!("missing next block"))?;
                if opcodes.last().is_some_and(|o| o.yields()) && !context.warp {
                    crate::log("next block, yielding required");
                    opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                        mode: YieldMode::Schedule(Rc::downgrade(&Step::from_block(
                            next_block,
                            next_id.clone(),
                            blocks,
                            context.clone(),
                            Weak::clone(&project),
                            final_next_blocks.clone(),
                        )?)),
                    }));
                    None
                } else {
                    crate::log("next block, no yielding required");
                    next_block.block_info()
                }
            } else if let (Some(popped_next), new_next_blocks_stack) =
                final_next_blocks.clone().pop_inner()
            {
                let next_block = blocks
                    .get(&popped_next.id)
                    .ok_or(make_hq_bad_proj!("missing next block"))?;
                if (popped_next.yield_first || opcodes.last().is_some_and(|o| o.yields()))
                    && !context.warp
                {
                    crate::log("next block, yielding required");
                    opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                        mode: YieldMode::Schedule(Rc::downgrade(&Step::from_block(
                            next_block,
                            popped_next.id.clone(),
                            blocks,
                            context.clone(),
                            Weak::clone(&project),
                            new_next_blocks_stack,
                        )?)),
                    }));
                    None
                } else {
                    crate::log("next block, no yielding required");
                    next_block.block_info()
                }
            } else if final_next_blocks == NextBlocks::NothingAtAll {
                crate::log("no next block; nothing at all");
                None
            } else {
                crate::log("no next block");
                opcodes.push(IrOpcode::hq_yield(HqYieldFields {
                    mode: YieldMode::None,
                }));
                None
            }
        } else {
            break;
        }
    }
    Ok(opcodes.into_iter().collect())
}

fn from_special_block(block_array: &BlockArray, context: &StepContext) -> HQResult<IrOpcode> {
    Ok(match block_array {
        BlockArray::NumberOrAngle(ty, value) => match ty {
            4 | 5 | 8 => IrOpcode::hq_float(HqFloatFields(*value)),
            6 | 7 => IrOpcode::hq_integer(HqIntegerFields(*value as i32)),
            _ => hq_bad_proj!("bad project json (block array of type ({}, f64))", ty),
        },
        BlockArray::ColorOrString(ty, value) => match ty {
            4 | 5 | 8 => {
                IrOpcode::hq_float(HqFloatFields(value.parse().map_err(|_| make_hq_bug!(""))?))
            }
            6 | 7 => IrOpcode::hq_integer(HqIntegerFields(
                value.parse().map_err(|_| make_hq_bug!(""))?,
            )),
            9 => hq_todo!(""),
            10 => IrOpcode::hq_text(HqTextFields(value.clone())),
            _ => hq_bad_proj!("bad project json (block array of type ({}, string))", ty),
        },
        BlockArray::Broadcast(ty, _name, id) | BlockArray::VariableOrList(ty, _name, id, _, _) => {
            match ty {
                12 => {
                    let target = context
                        .target
                        .upgrade()
                        .ok_or(make_hq_bug!("couldn't upgrade Weak"))?;
                    let variable = if let Some(var) = target.variables().get(id) {
                        var
                    } else {
                        hq_todo!("global variables")
                    };
                    IrOpcode::data_variable(DataVariableFields(RcVar(Rc::clone(variable))))
                }
                _ => hq_todo!(""),
            }
        }
    })
}
