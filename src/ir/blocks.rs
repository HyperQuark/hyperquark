use crate::instructions::{fields, IrOpcode};
use crate::prelude::*;
use crate::sb3::{Block, BlockArray, BlockInfo, BlockMap, BlockOpcode};
use fields::*;

pub fn from_block(block: &Block, blocks: &BlockMap) -> HQResult<Box<[IrOpcode]>> {
    Ok(match block {
        Block::Normal { block_info, .. } => from_normal_block(block_info, blocks)?,
        Block::Special(block_array) => Box::new([from_special_block(block_array)?]),
    })
}

fn from_normal_block(block_info: &BlockInfo, _blocks: &BlockMap) -> HQResult<Box<[IrOpcode]>> {
    Ok(match &block_info.opcode {
        BlockOpcode::operator_add => [IrOpcode::operator_add].into_iter(),
        BlockOpcode::looks_say => [IrOpcode::looks_say].into_iter(),
        other => hq_todo!("unimplemented block: {:?}", other),
    }
    .collect())
}

fn from_special_block(block_array: &BlockArray) -> HQResult<IrOpcode> {
    Ok(match block_array {
        BlockArray::NumberOrAngle(ty, value) => match ty {
            4 | 8 => IrOpcode::math_number(MathNumberFields(*value)),
            5 => IrOpcode::math_positive_number(MathPositiveNumberFields(*value)),
            6 => IrOpcode::math_whole_number(MathWholeNumberFields(*value as i64)),
            7 => IrOpcode::math_integer(MathIntegerFields(*value as i64)),
            _ => hq_bad_proj!("bad project json (block array of type ({}, f64))", ty),
        },
        BlockArray::ColorOrString(ty, value) => match ty {
            4 | 8 => IrOpcode::math_number(MathNumberFields(
                value.parse().map_err(|_| make_hq_bug!(""))?,
            )),
            5 => IrOpcode::math_positive_number(MathPositiveNumberFields(
                value.parse().map_err(|_| make_hq_bug!(""))?,
            )),
            6 => IrOpcode::math_whole_number(MathWholeNumberFields(
                value.parse().map_err(|_| make_hq_bug!(""))?,
            )),
            7 => IrOpcode::math_integer(MathIntegerFields(
                value.parse().map_err(|_| make_hq_bug!(""))?,
            )),
            9 => hq_todo!(""),
            10 => hq_todo!(), /*IrOpcode::text {
            TEXT: value.to_string(),
            },*/
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
