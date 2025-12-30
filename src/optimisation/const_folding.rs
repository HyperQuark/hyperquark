use super::SSAToken;
use crate::instructions::{
    ControlIfElseFields, ControlLoopFields, HqBooleanFields, HqFloatFields, HqIntegerFields,
    HqTextFields, HqYieldFields, IrOpcode, YieldMode,
};
use crate::ir::{IrProject, ReturnType, Step, Type as IrType};
use crate::prelude::*;

#[derive(Debug, Clone)]
pub enum ConstFoldItem {
    Float(f64),
    Int(i32),
    Bool(bool),
    String(Box<str>),
    Unknown {
        possible_types: IrType,
        opcodes: Rc<[IrOpcode]>,
    },
}

impl ConstFoldItem {
    pub fn possible_types(&self) -> HQResult<IrType> {
        Ok(match self {
            Self::Unknown { possible_types, .. } => *possible_types,
            Self::Float(f) => IrOpcode::hq_float(HqFloatFields(*f))
                .output_type(Rc::from([]))?
                .singleton_or_else(|| make_hq_bug!("got non-singleton output type for const"))?,
            Self::Int(i) => IrOpcode::hq_integer(HqIntegerFields(*i))
                .output_type(Rc::from([]))?
                .singleton_or_else(|| make_hq_bug!("got non-singleton output type for const"))?,
            Self::Bool(b) => IrOpcode::hq_boolean(HqBooleanFields(*b))
                .output_type(Rc::from([]))?
                .singleton_or_else(|| make_hq_bug!("got non-singleton output type for const"))?,
            Self::String(s) => IrOpcode::hq_text(HqTextFields(s.clone()))
                .output_type(Rc::from([]))?
                .singleton_or_else(|| make_hq_bug!("got non-singleton output type for const"))?,
        })
    }

    #[must_use]
    pub fn to_opcodes(&self) -> Rc<[IrOpcode]> {
        match self {
            Self::Unknown { opcodes, .. } => Rc::clone(opcodes),
            Self::Float(f) => Rc::from([IrOpcode::hq_float(HqFloatFields(*f))]),
            Self::Int(i) => Rc::from([IrOpcode::hq_integer(HqIntegerFields(*i))]),
            Self::Bool(b) => Rc::from([IrOpcode::hq_boolean(HqBooleanFields(*b))]),
            Self::String(s) => Rc::from([IrOpcode::hq_text(HqTextFields(s.clone()))]),
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

    let mut const_stack = vec![];

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

        if let IrOpcode::hq_yield(HqYieldFields {
            mode: YieldMode::Inline(inline_step),
        }) = opcode
        {
            const_fold_step(inline_step, state)?;
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
