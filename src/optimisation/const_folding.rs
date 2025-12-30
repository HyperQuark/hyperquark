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

fn const_fold_step(step: &Rc<Step>, state: &mut ConstFoldState) -> HQResult<()> {
    let mut new_opcodes = vec![];

    let mut const_stack: Vec<ConstFoldItem> = vec![];

    for opcode in step.opcodes().borrow().iter() {
        if let IrOpcode::control_if_else(ControlIfElseFields {
            branch_if,
            branch_else,
        }) = opcode
        {
            const_fold_step(branch_if, state)?;
            const_fold_step(branch_else, state)?;
        }

        if let IrOpcode::control_loop(ControlLoopFields {
            condition,
            first_condition,
            body,
            ..
        }) = opcode
        {
            const_fold_step(body, &mut ConstFoldState::default())?;
            const_fold_step(condition, &mut ConstFoldState::default())?;
            if let Some(first_cond_step) = first_condition.as_ref() {
                const_fold_step(first_cond_step, &mut ConstFoldState::default())?;
            }
        }

        if let IrOpcode::hq_yield(HqYieldFields { mode }) = opcode {
            if let YieldMode::Inline(inline_step) = mode {
                const_fold_step(inline_step, state)?;
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

    *step.opcodes_mut()? = new_opcodes;

    Ok(())
}

pub fn const_fold(proj: &Rc<IrProject>, _ssa_token: SSAToken) -> HQResult<()> {
    for step in proj.steps().borrow().iter() {
        const_fold_step(step, &mut ConstFoldState::default())?;
    }

    Ok(())
}
