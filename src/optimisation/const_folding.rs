use core::ops::Deref;

use super::SSAToken;
use crate::instructions::{
    ControlIfElseFields, ControlLoopFields, HqBoxFields, HqYieldFields, IrOpcode, YieldMode,
};
use crate::ir::{IrProject, ReturnType, Step, Type as IrType, var_val_instruction, var_val_type};
use crate::prelude::*;
use crate::sb3::VarVal;

#[derive(Debug, Clone)]
pub enum ConstFoldItem {
    Basic(VarVal),
    Boxed(VarVal, IrType),
    Stack(Rc<[IrOpcode]>),
    Unknown {
        possible_types: IrType,
        opcodes: Rc<[IrOpcode]>,
    },
}

impl ConstFoldItem {
    pub fn possible_types(&self) -> HQResult<IrType> {
        Ok(match self {
            Self::Unknown { possible_types, .. } => *possible_types,
            Self::Stack(_) => hq_bug!("called possible_types on ConstFoldItem::Stack"),
            Self::Basic(val) => var_val_type(val)?,
            Self::Boxed(_, ty) => *ty,
        })
    }

    #[must_use]
    pub fn to_opcodes(&self) -> Rc<[IrOpcode]> {
        match self {
            Self::Unknown { opcodes, .. } => Rc::clone(opcodes),
            Self::Basic(val) => Rc::from([var_val_instruction(val)]),
            Self::Stack(stack) => Rc::clone(stack),
            Self::Boxed(val, ty) => Rc::from([
                var_val_instruction(val),
                IrOpcode::hq_box(HqBoxFields { output_ty: *ty }),
            ]),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConstFold {
    NotFoldable,
    Folded(Rc<[ConstFoldItem]>),
}

#[derive(Default)]
pub struct ConstFoldState {
    pub vars: BTreeMap<Box<str>, ConstFoldItem>,
}

impl ConstFoldState {
    fn merge(&mut self, other: Self) {
        for (var, _) in other.vars {
            self.vars.insert(
                var,
                ConstFoldItem::Unknown {
                    possible_types: IrType::none(), // this is ok because the unknown value is never actually used
                    opcodes: Rc::from([]),
                },
            );
        }
    }
}

fn const_fold_step<S>(step: S, state: &mut ConstFoldState) -> HQResult<()>
where
    S: Deref<Target = RefCell<Step>>,
{
    let mut new_opcodes = vec![];

    let mut const_stack: Vec<ConstFoldItem> = vec![];

    for oopcode in step.try_borrow()?.opcodes() {
        let mut opcode = oopcode.clone();
        if let IrOpcode::control_if_else(ControlIfElseFields {
            branch_if,
            branch_else,
        }) = opcode
        {
            let branch_if_mut = Rc::new(Rc::unwrap_or_clone(branch_if));
            let branch_else_mut = Rc::new(Rc::unwrap_or_clone(branch_else));
            const_fold_step(Rc::clone(&branch_if_mut), state)?;
            const_fold_step(Rc::clone(&branch_else_mut), state)?;

            opcode = IrOpcode::control_if_else(ControlIfElseFields {
                branch_if: branch_if_mut,
                branch_else: branch_else_mut,
            });
        }

        if let IrOpcode::control_loop(ControlLoopFields {
            condition,
            first_condition,
            body,
            flip_if,
        }) = opcode
        {
            let body_mut = Rc::new(Rc::unwrap_or_clone(body));
            let condition_mut = Rc::new(Rc::unwrap_or_clone(condition));
            let mut body_state = ConstFoldState::default();
            const_fold_step(Rc::clone(&body_mut), &mut body_state)?;
            state.merge(body_state);
            let mut condition_state = ConstFoldState::default();
            const_fold_step(Rc::clone(&condition_mut), &mut condition_state)?;
            state.merge(condition_state);
            let first_condition_mut = first_condition
                .map(|first_cond_step| -> HQResult<_> {
                    let first_cond_mut = Rc::new(Rc::unwrap_or_clone(first_cond_step));
                    let mut cond_state = ConstFoldState::default();
                    const_fold_step(Rc::clone(&first_cond_mut), &mut cond_state)?;
                    state.merge(cond_state);

                    Ok(first_cond_mut)
                })
                .transpose()?;

            opcode = IrOpcode::control_loop(ControlLoopFields {
                body: body_mut,
                condition: condition_mut,
                first_condition: first_condition_mut,
                flip_if,
            });
        }

        if let IrOpcode::hq_yield(HqYieldFields { ref mode }) = opcode {
            if let YieldMode::Inline(inline_step) = mode {
                let inline_step_mut = Rc::new(Rc::unwrap_or_clone(Rc::clone(inline_step)));
                const_fold_step(Rc::clone(&inline_step_mut), state)?;
                opcode = IrOpcode::hq_yield(HqYieldFields {
                    mode: YieldMode::Inline(inline_step_mut),
                });
            } else {
                for const_item in &const_stack {
                    new_opcodes.extend(const_item.to_opcodes().iter().cloned());
                }
                new_opcodes.push(opcode.clone());
                break;
            }
        }

        let inputs_len = opcode.acceptable_inputs()?.len();

        hq_assert!(const_stack.len() >= inputs_len);

        let const_inputs: Vec<_> = const_stack
            .splice((const_stack.len() - inputs_len)..const_stack.len(), vec![])
            .collect();

        let const_fold = opcode.const_fold(&const_inputs[..], state)?;

        if let ConstFold::Folded(folded) = const_fold {
            const_stack.extend(folded.iter().cloned());
        } else {
            let output_type = opcode.output_type(
                const_inputs
                    .iter()
                    .map(ConstFoldItem::possible_types)
                    .collect::<HQResult<_>>()?,
            )?;

            match output_type {
                ReturnType::None => {
                    for const_item in const_inputs {
                        new_opcodes.extend(const_item.to_opcodes().iter().cloned());
                    }
                    new_opcodes.push(opcode.clone());
                }
                ReturnType::Singleton(out_ty) => const_stack.push(ConstFoldItem::Unknown {
                    possible_types: out_ty,
                    opcodes: const_inputs
                        .iter()
                        .map(ConstFoldItem::to_opcodes)
                        .flat_map(|op| op.iter().cloned().collect::<Box<[_]>>())
                        .chain([opcode.clone()])
                        .collect(),
                }),
                ReturnType::MultiValue(out_tys) => {
                    for const_item in const_inputs {
                        new_opcodes.extend(const_item.to_opcodes().iter().cloned());
                    }
                    new_opcodes.push(opcode.clone());
                    const_stack.extend(out_tys.iter().map(|ty| ConstFoldItem::Unknown {
                        possible_types: *ty,
                        opcodes: Rc::from([]),
                    }));
                }
            }
        }
    }

    for remaining_const_item in const_stack {
        new_opcodes.extend(remaining_const_item.to_opcodes().iter().cloned());
    }

    *step.try_borrow_mut()?.opcodes_mut() = new_opcodes;

    Ok(())
}

pub fn const_fold(proj: &Rc<IrProject>, _ssa_token: SSAToken) -> HQResult<()> {
    for step in proj.steps().borrow().iter() {
        const_fold_step(step, &mut ConstFoldState::default())?;
    }

    Ok(())
}
