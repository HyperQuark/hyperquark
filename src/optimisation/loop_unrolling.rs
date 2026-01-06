use crate::instructions::{
    ControlIfElseFields, ControlLoopFields, HqYieldFields, IrOpcode, YieldMode,
};
use crate::ir::{IrProject, Step};
use crate::prelude::*;
use crate::wasm::WasmFlags;

fn unroll_loops_in_step(step: &Rc<Step>) -> HQResult<()> {
    let mut loops = vec![];

    for (i, opcode) in step.opcodes().borrow().iter().enumerate().rev() {
        if let Some(inline_steps) = opcode.inline_steps() {
            inline_steps.iter().try_for_each(unroll_loops_in_step)?;
        }

        if let IrOpcode::control_loop(loop_fields) = opcode {
            loops.push((i, loop_fields.clone()));
        }
    }

    let mut opcodes = step.opcodes_mut()?;

    for (
        i,
        ControlLoopFields {
            first_condition,
            condition,
            body,
            flip_if,
        },
    ) in loops
    {
        #[expect(clippy::range_plus_one, reason = "i don't like inclusive range")]
        opcodes.splice(
            i..(i + 1),
            if let Some(ref first_condition_step) = first_condition {
                first_condition_step.opcodes().borrow()
            } else {
                condition.opcodes().borrow()
            }
            .iter()
            .cloned()
            .chain(if flip_if {
                vec![IrOpcode::operator_not]
            } else {
                vec![]
            })
            .chain(vec![IrOpcode::control_if_else(ControlIfElseFields {
                branch_if: Step::new_rc(
                    None,
                    step.context().clone(),
                    vec![
                        IrOpcode::hq_yield(HqYieldFields {
                            mode: YieldMode::Inline(Step::clone(&body, false)?),
                        }),
                        IrOpcode::control_loop(ControlLoopFields {
                            first_condition: None,
                            condition: condition.clone(),
                            body,
                            flip_if,
                        }),
                    ],
                    &step.project(),
                    false,
                )?,
                branch_else: Step::new_empty(
                    &step.project(),
                    false,
                    Rc::clone(step.context().target()),
                )?,
            })]),
        );
    }

    Ok(())
}

pub fn unroll_loops(proj: &Rc<IrProject>, flags: &WasmFlags) -> HQResult<()> {
    // todo: fine-tune this number, or somehow converge until no more improvements can be made
    for _ in 0..flags.unroll_loops {
        for step in proj.steps().borrow().iter() {
            unroll_loops_in_step(step)?;
        }
    }

    Ok(())
}
