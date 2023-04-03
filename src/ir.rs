// intermediate representation
use crate::sb3::{Block, BlockType, BlockOpcode, BlockOpcodeWithField, BlockArray, Input, BlockArrayOrId};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum ThreadStart {
    GreenFlag,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step {
    opcodes: Vec<BlockOpcodeWithField>,
    index: u32,
}

impl Step {
    pub fn new(opcodes: Vec<BlockOpcodeWithField>, index: u32) -> Self {
        Step { opcodes, index }
    }
    pub fn opcodes(&self) -> &Vec<BlockOpcodeWithField> {
        &self.opcodes
    }
    pub fn index(&self) -> &u32 {
        &self.index
    }
}

pub struct ThreadContext {
    pub target_index: u32,
    pub dbg: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Thread {
    start: ThreadStart,
    steps: Vec<Step>,
}

impl Thread {
    pub fn new(start: ThreadStart, steps: Vec<Step>) -> Self {
        Thread { start, steps }
    }
    pub fn start(&self) -> &ThreadStart {
        &self.start
    }
    pub fn steps(&self) -> &Vec<Step> {
        &self.steps
    }
    pub fn from_hat(hat: Block, blocks: BTreeMap<String, Block>, first_func_index: u32, context: &ThreadContext) -> Self {
        let mut ops: Vec<BlockOpcodeWithField> = vec![];
        fn add_block(block: Block, blocks: &BTreeMap<String, Block>, ops: &mut Vec<BlockOpcodeWithField>) {
            match block {
                Block::Normal { block_info, .. } => {
                    for (_name, input) in block_info.inputs {
                        match input {
                            Input::Shadow(_, maybe_block, _) | Input::NoShadow(_, maybe_block) => {
                                if let Some(block) = maybe_block {
                                    match block {
                                        BlockArrayOrId::Id(id) => {
                                            if let Some(actual_block) = blocks.get(&id) {
                                                add_block(actual_block.clone(), blocks, ops);
                                            }
                                        },
                                        BlockArrayOrId::Array(arr) => {
                                            add_block(Block::Special(arr), blocks, ops);
                                        }
                                    }
                                }
                            }
                        }
                    }
                    
                    ops.push(match block_info.opcode {
                        BlockOpcode::looks_say => BlockOpcodeWithField::looks_say,
                        BlockOpcode::looks_think => BlockOpcodeWithField::looks_think,
                        BlockOpcode::operator_add => BlockOpcodeWithField::operator_add,
                        BlockOpcode::operator_subtract => BlockOpcodeWithField::operator_subtract,
                        BlockOpcode::operator_multiply => BlockOpcodeWithField::operator_multiply,
                        BlockOpcode::operator_divide => BlockOpcodeWithField::operator_divide,
                        BlockOpcode::operator_mod => BlockOpcodeWithField::operator_mod,
                        BlockOpcode::operator_round => BlockOpcodeWithField::operator_round,
                        _ => todo!(),
                    });
                    
                    if let Some(next_id) = &block_info.next {
                        if let Some(next_block) = blocks.get(next_id) {
                            add_block(next_block.clone(), blocks, ops);
                        }
                    }
                    
                },
                Block::Special(a) => match a {
                    BlockArray::NumberOrAngle(ty, value) => ops.push(match ty {
                        4 => BlockOpcodeWithField::math_number { NUM: value.clone() },
                        5 => BlockOpcodeWithField::math_positive_number { NUM: value.clone() },
                        6 => BlockOpcodeWithField::math_whole_number { NUM: value.clone() },
                        7 => BlockOpcodeWithField::math_integer { NUM: value.clone() },
                        8 => BlockOpcodeWithField::math_angle { NUM: value.clone() },
                        _ => panic!("bad project json (block array of type ({}, u32))", ty),
                    }),
                    BlockArray::ColorOrString(ty, value) => ops.push(match ty {
                        4 => BlockOpcodeWithField::math_number { NUM: value.parse().unwrap() },
                        5 => BlockOpcodeWithField::math_positive_number { NUM: value.parse().unwrap() },
                        6 => BlockOpcodeWithField::math_whole_number { NUM: value.parse().unwrap() },
                        7 => BlockOpcodeWithField::math_integer { NUM: value.parse().unwrap() },
                        8 => BlockOpcodeWithField::math_angle { NUM: value.parse().unwrap() },
                        9 => todo!(),
                        10 => BlockOpcodeWithField::math_number { NUM: value.parse().unwrap() }, // this is for testing purposes, will change later
                        _ => panic!("bad project json (block array of type ({}, string))", ty),
                    }),
                    BlockArray::Broadcast(ty, _name, _id) => match ty {
                        _ => todo!(),
                    },
                    BlockArray::VariableOrList(ty, _name, _id, _pos_x, _pos_y) => match ty {
                        _ => todo!(),
                    },
                },
            };
        }
        if let Block::Normal { block_info, .. } = &hat {
            if let Some(next_id) = &block_info.next {
                if let Some(next_block) = blocks.get(next_id) {
                    add_block(next_block.clone(), &blocks, &mut ops);
                }
            }
        }
        let start_type = if let Block::Normal { block_info, .. } = &hat {
            match block_info.opcode {
                BlockOpcode::event_whenflagclicked => ThreadStart::GreenFlag,
                _ => todo!(),
            }
        } else {
            unreachable!()
        };
        let mut steps: Vec<Step> = vec![];
        //let mut lastOpRequestedRedraw = false;
        let mut this_step_ops: Vec<BlockOpcodeWithField> = vec![];
        for op in ops {
            this_step_ops.push(op.clone());
            if op.does_request_redraw() && !(op == BlockOpcodeWithField::looks_say && context.dbg) {
                let steps_len: u32 = steps.len().try_into().expect("step count out of bounds (E004)");
                steps.push(Step::new(this_step_ops.clone(), first_func_index + steps_len));
                this_step_ops = vec![];
            }
        }
        Self::new(start_type, steps)
    }
}