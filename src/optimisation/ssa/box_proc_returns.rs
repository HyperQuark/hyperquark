use core::ops::Deref;

use crate::instructions::{
    DataVariableFields, HqBoxFields, HqYieldFields, IrOpcode, ProceduresArgumentFields, YieldMode,
};
use crate::ir::{RcVar, Step};
use crate::prelude::*;

pub fn box_proc_returns<S>(
    step: S,
    rev_ret_vars: &[&RcVar],
    visited_steps: &mut BTreeSet<Box<str>>,
) -> HQResult<()>
where
    S: Deref<Target = RefCell<Step>>,
{
    if visited_steps.contains(step.try_borrow()?.id()) {
        return Ok(());
    }

    visited_steps.insert(step.try_borrow()?.id().into());

    let opcodes_len = step.try_borrow()?.opcodes().len();

    for i in 0..opcodes_len {
        let inlined_steps = step
            .try_borrow()?
            .opcodes()
            .get(i)
            .ok_or_else(|| make_hq_bug!("opcode index out of bounds"))?
            .inline_steps(true);
        if let Some(inline_steps) = inlined_steps {
            let new_inline_steps = inline_steps
                .iter()
                .map(|inline_step| {
                    let inline_step_mut = Rc::new(Rc::unwrap_or_clone(Rc::clone(inline_step)));
                    box_proc_returns(Rc::clone(&inline_step_mut), rev_ret_vars, visited_steps)?;
                    Ok(inline_step_mut)
                })
                .collect::<HQResult<Box<[_]>>>()?;
            for (step_ref_mut, new_inline_step) in step
                .try_borrow_mut()?
                .opcodes_mut()
                .get_mut(i)
                .ok_or_else(|| make_hq_bug!("opcode index out of bounds"))?
                .inline_steps_mut(true)
                .ok_or_else(|| {
                    make_hq_bug!("inline_steps_mut was None when inline_steps was Some")
                })?
                .iter_mut()
                .zip(new_inline_steps)
            {
                **step_ref_mut = new_inline_step;
            }
        }
    }

    let mut box_additions = Vec::new();

    let mut rev_vars_iter = rev_ret_vars.iter();

    let mut to_skip = 0;

    for (i, opcode) in step
        .try_borrow()?
        .opcodes()
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, op)| {
            !matches!(
                op,
                IrOpcode::hq_yield(HqYieldFields {
                    mode: YieldMode::Return,
                }),
            )
        })
    {
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "too many variants to match explicitly"
        )]
        match opcode {
            IrOpcode::data_variable(DataVariableFields { var, .. }) => {
                if to_skip > 0 {
                    to_skip -= 1;
                    continue;
                }
                let Some(ret_var) = rev_vars_iter.next() else {
                    break;
                };
                if var.borrow().possible_types().is_base_type()
                    && !ret_var.possible_types().is_base_type()
                {
                    box_additions.push((i + 1, *ret_var.possible_types()));
                }
            }
            IrOpcode::procedures_argument(ProceduresArgumentFields { arg_var, .. }) => {
                if to_skip > 0 {
                    to_skip -= 1;
                    continue;
                }
                let Some(ret_var) = rev_vars_iter.next() else {
                    break;
                };
                if arg_var.possible_types().is_base_type()
                    && !ret_var.possible_types().is_base_type()
                {
                    box_additions.push((i + 1, *ret_var.possible_types()));
                }
            }
            IrOpcode::data_setvariableto(_) => {
                to_skip += 1;
            }
            _ => {
                break;
            }
        }
    }

    let mut step_mut = step.try_borrow_mut()?;
    let opcodes_mut = step_mut.opcodes_mut();

    for (box_addition, output_ty) in box_additions {
        opcodes_mut.splice(
            box_addition..box_addition,
            vec![IrOpcode::hq_box(HqBoxFields { output_ty })],
        );
    }

    Ok(())
}
