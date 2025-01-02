use crate::instructions::{IrOpcode, MathNumberFields};
use crate::prelude::*;
use crate::sb3::{Block, BlockArray, BlockInfo, BlockMap, BlockOpcode};

impl BlockOpcode {
    fn from_block(block: &Block, blocks: &BlockMap) -> HQResult<IrOpcode> {
        match block {
            Block::Normal { block_info, .. } => BlockOpcode::from_normal_block(block_info, blocks),
            Block::Special(block_array) => BlockOpcode::from_special_block(block_array, blocks),
        }
    }

    fn from_normal_block(block_info: &BlockInfo, blocks: &BlockMap) -> HQResult<IrOpcode> {
        hq_todo!()
    }

    fn from_special_block(block_array: &BlockArray, blocks: &BlockMap) -> HQResult<IrOpcode> {
        Ok(match block_array {
            BlockArray::NumberOrAngle(ty, value) => match ty {
                4 => IrOpcode::math_number(MathNumberFields(*value)),
                //5 => IrOpcode::math_positive_number { NUM: *value as i32 },
                //6 => IrOpcode::math_whole_number { NUM: *value as i32 },
                //7 => IrOpcode::math_integer { NUM: *value as i32 },
                //8 => IrOpcode::math_angle { NUM: *value as i32 },
                _ => hq_bad_proj!("bad project json (block array of type ({}, f64))", ty),
            },
            BlockArray::ColorOrString(ty, value) => match ty {
                /*4 => IrOpcode::math_number {
                    NUM: value.parse().map_err(|_| make_hq_bug!(""))?,
                },*/
                /*5 => IrOpcode::math_positive_number {
                    NUM: value.parse().map_err(|_| make_hq_bug!(""))?,
                },
                6 => IrOpcode::math_whole_number {
                    NUM: value.parse().map_err(|_| make_hq_bug!(""))?,
                },
                7 => IrOpcode::math_integer {
                    NUM: value.parse().map_err(|_| make_hq_bug!(""))?,
                },
                8 => IrOpcode::math_angle {
                    NUM: value.parse().map_err(|_| make_hq_bug!(""))?,
                },
                9 => hq_todo!(""),
                10 => IrOpcode::text {
                    TEXT: value.to_string(),
                },*/
                _ => hq_bad_proj!("bad project json (block array of type ({}, string))", ty),
            },
            BlockArray::Broadcast(ty, _name, id) | BlockArray::VariableOrList(ty, _name, id, _, _) => match ty {
                /*12 => IrOpcode::data_variable {
                    VARIABLE: id.to_string(),
                    assume_type: None,
                },*/
                _ => hq_todo!(""),
            },
        })
    }
}
