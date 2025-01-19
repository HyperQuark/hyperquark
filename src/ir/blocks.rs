use crate::instructions::{fields::*, IrOpcode};
use crate::prelude::*;
use crate::sb3::{Block, BlockArray, BlockArrayOrId, BlockInfo, BlockMap, BlockOpcode, Input};

// TODO: insert casts in relevant places

pub fn from_block(block: &Block, blocks: &BlockMap) -> HQResult<Box<[IrOpcode]>> {
    Ok(match block {
        Block::Normal { block_info, .. } => {
            if let Some(next_id) = &block_info.next {
                from_normal_block(block_info, blocks)?
                    .iter()
                    .chain(
                        from_block(
                            blocks
                                .get(next_id)
                                .ok_or(make_hq_bad_proj!("specified next block missing"))?,
                            blocks,
                        )?
                        .iter(),
                    )
                    .cloned()
                    .collect()
            } else {
                from_normal_block(block_info, blocks)?
                    .iter()
                    .chain([IrOpcode::hq__yield(HqYieldFields(None))].iter())
                    .cloned()
                    .collect()
            }
        }
        Block::Special(block_array) => Box::new([from_special_block(block_array)?]),
    })
}

pub fn input_names(opcode: BlockOpcode) -> HQResult<Vec<String>> {
    Ok(match opcode {
        BlockOpcode::looks_say => vec!["MESSAGE"],
        BlockOpcode::operator_add => vec!["NUM1", "NUM2"],
        other => hq_todo!("unimplemented input_names for {:?}", other),
    }
    .into_iter()
    .map(String::from)
    .collect())
}

pub fn inputs(block_info: &BlockInfo, blocks: &BlockMap) -> HQResult<Vec<IrOpcode>> {
    Ok(input_names(block_info.opcode.clone())?
        .into_iter()
        .map(|name| -> HQResult<Vec<IrOpcode>> {
            match block_info
                .inputs
                .get((*name).into())
                .ok_or(make_hq_bad_proj!("missing input {}", name))?
            {
                Input::NoShadow(_, Some(block)) | Input::Shadow(_, Some(block), _) => match block {
                    BlockArrayOrId::Array(arr) => Ok(vec![from_special_block(arr)?]),
                    BlockArrayOrId::Id(id) => match blocks
                        .get(id)
                        .ok_or(make_hq_bad_proj!("block for input {} doesn't exist", name))?
                    {
                        Block::Normal { block_info, .. } => {
                            Ok(from_normal_block(block_info, blocks)?.into())
                        }
                        Block::Special(block_array) => Ok(vec![from_special_block(block_array)?]),
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

fn from_normal_block(block_info: &BlockInfo, blocks: &BlockMap) -> HQResult<Box<[IrOpcode]>> {
    Ok(inputs(block_info, blocks)?
        .into_iter()
        .chain(match &block_info.opcode {
            BlockOpcode::operator_add => [IrOpcode::operator_add].into_iter(),
            BlockOpcode::looks_say => [IrOpcode::looks_say].into_iter(),
            other => hq_todo!("unimplemented block: {:?}", other),
        })
        .collect())
}

fn from_special_block(block_array: &BlockArray) -> HQResult<IrOpcode> {
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
        BlockArray::Broadcast(ty, _name, _id)
        | BlockArray::VariableOrList(ty, _name, _id, _, _) => match ty {
            /*12 => IrOpcode::data_variable {
                VARIABLE: id.to_string(),
                assume_type: None,
            },*/
            _ => hq_todo!(""),
        },
    })
}
