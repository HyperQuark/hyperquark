use super::{RcVar, Type as IrType};
use crate::instructions::{fields::*, IrOpcode};
use crate::prelude::*;
use crate::sb3;
use sb3::{Block, BlockArray, BlockArrayOrId, BlockInfo, BlockMap, BlockOpcode, Input};

use super::context::StepContext;

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
            if !expected.contains(actual.0) {
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

pub fn from_block(
    block: &Block,
    blocks: &BlockMap,
    context: &StepContext,
) -> HQResult<Vec<IrOpcode>> {
    insert_casts(match block {
        Block::Normal { block_info, .. } => {
            if let Some(next_id) = &block_info.next {
                from_normal_block(block_info, blocks, context)?
                    .iter()
                    .chain(
                        from_block(
                            blocks
                                .get(next_id)
                                .ok_or(make_hq_bad_proj!("specified next block missing"))?,
                            blocks,
                            context,
                        )?
                        .iter(),
                    )
                    .cloned()
                    .collect()
            } else {
                from_normal_block(block_info, blocks, context)?
                    .iter()
                    .chain([IrOpcode::hq__yield(HqYieldFields(None))].iter())
                    .cloned()
                    .collect()
            }
        }
        Block::Special(block_array) => vec![from_special_block(block_array, context)?],
    })
}

pub fn input_names(opcode: BlockOpcode) -> HQResult<Vec<String>> {
    Ok(match opcode {
        BlockOpcode::looks_say => vec!["MESSAGE"],
        BlockOpcode::operator_add
        | BlockOpcode::operator_divide
        | BlockOpcode::operator_subtract
        | BlockOpcode::operator_multiply => vec!["NUM1", "NUM2"],
        BlockOpcode::operator_lt => vec!["OPERAND1", "OPERAND2"],
        BlockOpcode::operator_join => vec!["STRING1", "STRING2"],
        BlockOpcode::sensing_dayssince2000 | BlockOpcode::data_variable => vec![],
        BlockOpcode::data_setvariableto => vec!["VALUE"],
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
) -> HQResult<Vec<IrOpcode>> {
    Ok(input_names(block_info.opcode.clone())?
        .into_iter()
        .map(|name| -> HQResult<Vec<IrOpcode>> {
            match block_info
                .inputs
                .get((*name).into())
                .ok_or(make_hq_bad_proj!("missing input {}", name))?
            {
                Input::NoShadow(_, Some(block)) | Input::Shadow(_, Some(block), _) => match block {
                    BlockArrayOrId::Array(arr) => Ok(vec![from_special_block(arr, context)?]),
                    BlockArrayOrId::Id(id) => match blocks
                        .get(id)
                        .ok_or(make_hq_bad_proj!("block for input {} doesn't exist", name))?
                    {
                        Block::Normal { block_info, .. } => {
                            Ok(from_normal_block(block_info, blocks, context)?.into())
                        }
                        Block::Special(block_array) => {
                            Ok(vec![from_special_block(block_array, context)?])
                        }
                    },
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

fn from_normal_block(
    block_info: &BlockInfo,
    blocks: &BlockMap,
    context: &StepContext,
) -> HQResult<Box<[IrOpcode]>> {
    Ok(inputs(block_info, blocks, context)?
        .into_iter()
        .chain(match &block_info.opcode {
            BlockOpcode::operator_add => [IrOpcode::operator_add].into_iter(),
            BlockOpcode::operator_subtract => [IrOpcode::operator_subtract].into_iter(),
            BlockOpcode::operator_multiply => [IrOpcode::operator_multiply].into_iter(),
            BlockOpcode::operator_divide => [IrOpcode::operator_divide].into_iter(),
            BlockOpcode::looks_say => [IrOpcode::looks_say].into_iter(),
            BlockOpcode::operator_join => [IrOpcode::operator_join].into_iter(),
            BlockOpcode::sensing_dayssince2000 => [IrOpcode::sensing_dayssince2000].into_iter(),
            BlockOpcode::operator_lt => [IrOpcode::operator_lt].into_iter(),
            BlockOpcode::data_setvariableto => {
                let sb3::Field::ValueId(_val, maybe_id) = block_info.fields.get("VARIABLE").ok_or(
                    make_hq_bad_proj!("invalid project.json - missing field VARIABLE"),
                )?
                else {
                    hq_bad_proj!("invalid project.json - missing variable id for VARIABLE field");
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
                [IrOpcode::data_setvariableto(DataSetvariabletoFields(
                    RcVar(Rc::clone(variable)),
                ))]
                .into_iter()
            }
            BlockOpcode::data_variable => {
                let sb3::Field::ValueId(_val, maybe_id) = block_info.fields.get("VARIABLE").ok_or(
                    make_hq_bad_proj!("invalid project.json - missing field VARIABLE"),
                )?
                else {
                    hq_bad_proj!("invalid project.json - missing variable id for VARIABLE field");
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
                [IrOpcode::data_variable(DataVariableFields(RcVar(
                    Rc::clone(variable),
                )))]
                .into_iter()
            }
            other => hq_todo!("unimplemented block: {:?}", other),
        })
        .collect())
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
