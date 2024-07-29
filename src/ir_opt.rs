use crate::ir::{InputType, IrBlock, IrOpcode, IrProject, IrVal, TypeStack, TypeStackImpl};
use crate::log;
use crate::HQError;
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

impl IrProject {
    pub fn optimise(&mut self) -> Result<(), HQError> {
        self.const_fold()?;
        //self.variable_types()?;
        Ok(())
    }
    pub fn variable_types(&mut self) -> Result<(), HQError> {
        let mut var_map: BTreeMap<String, BTreeSet<InputType>> = BTreeMap::new();
        let mut block_swaps: BTreeMap<
            String,
            BTreeMap<
                (String, String),
                Vec<(usize, Option<(IrOpcode, Rc<RefCell<Option<TypeStack>>>)>)>,
            >,
        > = BTreeMap::new(); // here Some(_) means swap the block, None means remove the block
        for (step_identifier, step) in self.steps.clone() {
            for (i, opcode) in step.opcodes().iter().enumerate() {
                match opcode.opcode() {
                    IrOpcode::data_setvariableto { VARIABLE, .. }
                    | IrOpcode::data_teevariable { VARIABLE, .. } => {
                        let var_type_set = var_map.entry(VARIABLE.clone()).or_default();
                        if i == 0 {
                            hq_bug!("found a data_setvariableto or data_teevariable in position 0");
                        }
                        let previous_block = step.opcodes().get(i - 1).unwrap();
                        let input_type = match previous_block.opcode() {
                            IrOpcode::hq_cast(from, InputType::Unknown) => {
                                block_swaps
                                    .entry(VARIABLE.clone())
                                    .or_default()
                                    .entry(step_identifier.clone())
                                    .or_default()
                                    .push((i - 1, None));
                                from
                            }
                            IrOpcode::unknown_const { val } => &match val {
                                IrVal::Boolean(b) => {
                                    block_swaps
                                        .entry(VARIABLE.clone())
                                        .or_default()
                                        .entry(step_identifier.clone())
                                        .or_default()
                                        .push((
                                            i - 1,
                                            Some((
                                                IrOpcode::boolean { BOOL: *b },
                                                Rc::new(RefCell::new(Some(TypeStack(
                                                    previous_block.type_stack().get(1),
                                                    InputType::Boolean,
                                                )))),
                                            )),
                                        ));
                                    InputType::Boolean
                                }
                                IrVal::Int(i) => {
                                    block_swaps
                                        .entry(VARIABLE.clone())
                                        .or_default()
                                        .entry(step_identifier.clone())
                                        .or_default()
                                        .push((
                                            (i - 1).try_into().map_err(|_| {
                                                make_hq_bug!("i32 out of range for usize")
                                            })?,
                                            Some((
                                                IrOpcode::math_integer { NUM: *i },
                                                Rc::new(RefCell::new(Some(TypeStack(
                                                    previous_block.type_stack().get(1),
                                                    InputType::ConcreteInteger,
                                                )))),
                                            )),
                                        ));
                                    InputType::ConcreteInteger
                                }
                                IrVal::Float(f) => {
                                    block_swaps
                                        .entry(VARIABLE.clone())
                                        .or_default()
                                        .entry(step_identifier.clone())
                                        .or_default()
                                        .push((
                                            i - 1,
                                            Some((
                                                IrOpcode::math_number { NUM: *f },
                                                Rc::new(RefCell::new(Some(TypeStack(
                                                    previous_block.type_stack().get(1),
                                                    InputType::Float,
                                                )))),
                                            )),
                                        ));
                                    InputType::Float
                                }
                                IrVal::String(s) => {
                                    block_swaps
                                        .entry(VARIABLE.clone())
                                        .or_default()
                                        .entry(step_identifier.clone())
                                        .or_default()
                                        .push((
                                            i - 1,
                                            Some((
                                                IrOpcode::text { TEXT: s.clone() },
                                                Rc::new(RefCell::new(Some(TypeStack(
                                                    previous_block.type_stack().get(1),
                                                    InputType::String,
                                                )))),
                                            )),
                                        ));
                                    InputType::String
                                }
                                IrVal::Unknown(_) => {
                                    hq_bug!("unexpected unknown value inside of unknown const")
                                }
                            },
                            _ => &previous_block
                                .type_stack()
                                .borrow()
                                .clone()
                                .ok_or(make_hq_bug!("unexpected empty type stack"))?
                                .1
                                .clone(),
                        };
                        var_type_set.insert(input_type.clone());
                    }
                    _ => (),
                }
            }
        }
        for (var_id, types) in var_map {
            if types.len() == 1 {
                let ty = types.iter().next().unwrap();
                log(format!("found variable of single type {:?}", ty).as_str());
                for (step_identifier, mut step) in self.steps.clone() {
                    let maybe_swaps_from_this_var = block_swaps.get(&var_id);
                    if let Some(swaps_from_this_var) = maybe_swaps_from_this_var {
                        let to_swap = swaps_from_this_var.get(&step_identifier);
                        if to_swap.is_none() {
                            continue;
                        }
                        let Some(swap_vec) = to_swap else {
                            unreachable!()
                        };
                        for (index, swap) in swap_vec.iter().rev() {
                            if let Some((opcode, type_stack)) = swap {
                                *(step
                                    .opcodes_mut()
                                    .get_mut(*index)
                                    .ok_or(make_hq_bug!("swap index out of range")))? =
                                    IrBlock::new_with_stack_no_cast(
                                        opcode.clone(),
                                        type_stack.clone(),
                                    )?
                            } else {
                                step.opcodes_mut().remove(*index);
                            }
                        }
                    }
                    for opcode in step.opcodes_mut() {
                        match &mut opcode.opcode {
                            IrOpcode::data_setvariableto {
                                VARIABLE,
                                assume_type: ref mut assume_type @ None,
                            }
                            | IrOpcode::data_teevariable {
                                VARIABLE,
                                assume_type: ref mut assume_type @ None,
                            } if VARIABLE == &var_id => {
                                *assume_type = Some(ty.clone());
                            }
                            IrOpcode::data_variable {
                                VARIABLE,
                                assume_type: ref mut assume_type @ None,
                            } if VARIABLE == &var_id => {
                                *assume_type = Some(ty.clone());
                            }
                            _ => (),
                        }
                    }
                }
            }
        }
        Ok(())
    }
    pub fn const_fold(&mut self) -> Result<(), HQError> {
        for step in self.steps.values_mut() {
            step.const_fold()?;
        }
        Ok(())
    }
}
