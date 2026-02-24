use core::ops::Deref;

use crate::instructions::{ControlIfElseFields, ControlLoopFields, IrOpcode};
use crate::ir::{IrProject, Step};
use crate::prelude::*;
use crate::wasm::WasmFlags;

fn unroll_loops_in_step<S>(step: S) -> HQResult<()>
where
    S: Deref<Target = RefCell<Step>>,
{
    let mut loops = vec![];

    for (i, opcode) in step.try_borrow()?.opcodes().iter().enumerate().rev() {
        if let Some(inline_steps) = opcode.inline_steps(true) {
            inline_steps
                .iter()
                .cloned()
                .try_for_each(unroll_loops_in_step)?;
        }

        if let IrOpcode::control_loop(loop_fields) = opcode {
            loops.push((i, loop_fields.clone()));
        }
    }

    for (
        i,
        ControlLoopFields {
            first_condition,
            condition,
            ref body,
            flip_if,
            pre_body,
        },
    ) in loops
    {
        let initial_condition_opcodes = if let Some(ref first_condition_step) = first_condition {
            first_condition_step.try_borrow()?.opcodes().clone()
        } else {
            condition.try_borrow()?.opcodes().clone()
        };
        let replacement = initial_condition_opcodes
            .iter()
            .cloned()
            .chain(if flip_if {
                vec![IrOpcode::operator_not]
            } else {
                vec![]
            })
            .chain(vec![IrOpcode::control_if_else(ControlIfElseFields {
                branch_if: Rc::new(RefCell::new(Step::new(
                    None,
                    step.try_borrow()?.context().clone(),
                    pre_body
                        .as_ref()
                        .map_or_else(
                            || -> HQResult<_> { Ok(vec![]) },
                            |pb| Ok(pb.try_borrow()?.opcodes().clone()),
                        )?
                        .into_iter()
                        .chain(body.try_borrow()?.opcodes().clone())
                        .chain([IrOpcode::control_loop(ControlLoopFields {
                            first_condition: None,
                            condition: condition.clone(),
                            body: Rc::clone(body),
                            flip_if,
                            pre_body: pre_body.clone(),
                        })])
                        .collect(),
                    step.try_borrow()?.project(),
                    false,
                ))),
                branch_else: Rc::new(RefCell::new(Step::new_empty(
                    step.try_borrow()?.project(),
                    false,
                    Rc::clone(step.try_borrow()?.context().target()),
                ))),
            })]);
        #[expect(clippy::range_plus_one, reason = "i don't like inclusive range")]
        step.try_borrow_mut()?
            .opcodes_mut()
            .splice(i..(i + 1), replacement);
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
