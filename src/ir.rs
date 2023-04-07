// intermediate representation
use crate::sb3::{
    Block, BlockArray, BlockArrayOrId, BlockOpcode, BlockOpcodeWithField, BlockType, Field, Input, VariableInfo,
};
use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum ThreadStart {
    GreenFlag,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Step<'a> {
    opcodes: Vec<BlockOpcodeWithField>,
    index: u32,
    context: ThreadContext<'a>,
}

impl Step<'_> {
    pub fn new(opcodes: Vec<BlockOpcodeWithField>, index: u32, context: ThreadContext<'_>) -> Step<'_> {
        Step {
            opcodes,
            index,
            context,
        }
    }
    pub fn opcodes(&self) -> &Vec<BlockOpcodeWithField> {
        &self.opcodes
    }
    pub fn index(&self) -> &u32 {
        &self.index
    }
    pub fn context(&self) -> &ThreadContext {
        &self.context
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ThreadContext<'a> {
    pub target_index: u32,
    pub dbg: bool,
    pub vars: &'a Vec<&'a VariableInfo>, // hopefully there can't be two variables with the same is in differwnt sprites, otherwise this will break horrendously
}

#[derive(Debug, Clone, PartialEq)]
pub struct Thread<'a> {
    start: ThreadStart,
    steps: Vec<Step<'a>>,
}

impl Thread<'_> {
    pub fn new(start: ThreadStart, steps: Vec<Step<'_>>) -> Thread<'_> {
        Thread { start, steps }
    }
    pub fn start(&self) -> &ThreadStart {
        &self.start
    }
    pub fn steps(&self) -> &Vec<Step> {
        &self.steps
    }
    pub fn from_hat<'a>(
        hat: Block,
        blocks: BTreeMap<String, Block>,
        first_func_index: u32,
        context: &'a ThreadContext,
    ) -> Thread<'a> {
        let mut ops: Vec<BlockOpcodeWithField> = vec![];
        fn add_block(
            block: Block,
            blocks: &BTreeMap<String, Block>,
            ops: &mut Vec<BlockOpcodeWithField>,
        ) {
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
                                        }
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
                        BlockOpcode::data_variable => {
                            let Field::ValueId(_val, maybe_id) = block_info.fields.get("VARIABLE")
                                .expect("invalid project.json - missing field VARIABLE (E023)") else {
                                    panic!("invalid project.json - missing variable id for VARIABLE field (E024)");
                                };
                            let id = maybe_id.clone().expect("invalid project.json - null variable id for VARIABLE field (E025)");
                            BlockOpcodeWithField::data_variable {
                              VARIABLE: id,
                            }
                        },
                        BlockOpcode::data_setvariableto => {
                            let Field::ValueId(_val, maybe_id) = block_info.fields.get("VARIABLE")
                                .expect("invalid project.json - missing field VARIABLE (E026)") else {
                                    panic!("invalid project.json - missing variable id for VARIABLE field (E027)");
                                };
                            let id = maybe_id.clone().expect("invalid project.json - null variable id for VARIABLE field (E028)");
                            BlockOpcodeWithField::data_setvariableto {
                              VARIABLE: id,
                            }
                        },
                        BlockOpcode::data_changevariableby => {
                            let Field::ValueId(_val, maybe_id) = block_info.fields.get("VARIABLE")
                                .expect("invalid project.json - missing field VARIABLE (E029)") else {
                                    panic!("invalid project.json - missing variable id for VARIABLE field (E030)");
                                };
                            let id = maybe_id.clone().expect("invalid project.json - null variable id for VARIABLE field (E031)");
                            BlockOpcodeWithField::data_changevariableby {
                              VARIABLE: id,
                            }
                        },
                        
                        _ => todo!(),
                    });

                    if let Some(next_id) = &block_info.next {
                        if let Some(next_block) = blocks.get(next_id) {
                            add_block(next_block.clone(), blocks, ops);
                        }
                    }
                }
                Block::Special(a) => match a {
                    BlockArray::NumberOrAngle(ty, value) => ops.push(match ty {
                        4 => BlockOpcodeWithField::math_number { NUM: value },
                        5 => BlockOpcodeWithField::math_positive_number { NUM: value },
                        6 => BlockOpcodeWithField::math_whole_number { NUM: value },
                        7 => BlockOpcodeWithField::math_integer { NUM: value },
                        8 => BlockOpcodeWithField::math_angle { NUM: value },
                        _ => panic!("bad project json (block array of type ({}, u32))", ty),
                    }),
                    BlockArray::ColorOrString(ty, value) => ops.push(match ty {
                        4 => BlockOpcodeWithField::math_number {
                            NUM: value.parse().unwrap(),
                        },
                        5 => BlockOpcodeWithField::math_positive_number {
                            NUM: value.parse().unwrap(),
                        },
                        6 => BlockOpcodeWithField::math_whole_number {
                            NUM: value.parse().unwrap(),
                        },
                        7 => BlockOpcodeWithField::math_integer {
                            NUM: value.parse().unwrap(),
                        },
                        8 => BlockOpcodeWithField::math_angle {
                            NUM: value.parse().unwrap(),
                        },
                        9 => todo!(),
                        10 => BlockOpcodeWithField::text {
                            TEXT: value,
                        },
                        _ => panic!("bad project json (block array of type ({}, string))", ty),
                    }),
                    BlockArray::Broadcast(ty, _name, id) => ops.push(match ty {
                        12 => BlockOpcodeWithField::data_variable {
                            VARIABLE: id,
                        },
                        _ => todo!(),
                    }),
                    BlockArray::VariableOrList(ty, _name, id, _pos_x, _pos_y) => {
                        ops.push(match ty {
                            12 => BlockOpcodeWithField::data_variable {
                                VARIABLE: id,
                            },
                            _ => todo!(),
                        })
                    }
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
        let mut type_stack: Vec<(usize, BlockType)> = vec![];
        let mut needs_cast: Vec<(usize, BlockType)> = vec![];
        for (index, op) in ops.iter().enumerate() {
            assert!(
                type_stack.len() >= op.descriptor().inputs().len(),
                "type stack not big enough (expected >={} items, got {}) (E019)",
                op.descriptor().inputs().len(),
                type_stack.len()
            );
            for block_type in op.descriptor().inputs().iter().rev() {
                let top_type = type_stack
                    .pop()
                    .expect("couldn't pop from type stack (E020)");
                if block_type != &top_type.1 {
                    needs_cast.push((top_type.0, block_type.clone()));
                }
            }
            if !matches!(op.descriptor().output(), BlockType::Stack) {
                type_stack.push((index, (*op.descriptor().output()).clone()));
            }
        }
        for (index, block_type) in needs_cast.iter().rev() {
            use BlockOpcodeWithField::*;
            use BlockType::*;
            ops.insert(
                index + 1,
                match (
                    ops.get(*index)
                        .expect("couldn't find `ops` element which should exist (E021)")
                        .descriptor()
                        .output(),
                    block_type,
                ) {
                    (Text, Number) => cast_string_num,
                    (Text, Boolean) => cast_string_bool,
                    (Text, Any) => cast_string_any,
                    (Boolean, Number) => cast_bool_num,
                    (Boolean, Text) => cast_bool_string,
                    (Boolean, Any) => cast_bool_any,
                    (Number, Text) => cast_num_string,
                    (Number, Boolean) => cast_num_bool,
                    (Number, Any) => cast_num_any,
                    (Any, Text) => cast_any_string,
                    (Any, Boolean) => cast_any_bool,
                    (Any, Number) => cast_any_num,
                    _ => unreachable!(),
                },
            );
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
                let steps_len: u32 = steps
                    .len()
                    .try_into()
                    .expect("step count out of bounds (E004)");
                steps.push(Step::new(
                    this_step_ops.clone(),
                    first_func_index + steps_len,
                    context.clone(),
                ));
                this_step_ops = vec![];
            }
        }
        Self::new(start_type, steps)
    }
}
