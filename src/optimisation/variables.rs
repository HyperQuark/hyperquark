use crate::instructions::{
    ControlIfElseFields, ControlLoopFields, DataSetvariabletoFields, DataTeevariableFields,
    DataVariableFields, HqYieldFields, IrOpcode, YieldMode,
};
use crate::ir::{IrProject, PartialStep, RcVar, Step};
use crate::prelude::*;
use crate::sb3::VarVal;

/// Optimise variable types and locality.
///
/// Our three aims here are:
/// - ensure that variable access uses locals rather than globals, wherever possible;
/// - ensure that, within a step, local variable access is as type-constrained as possible;
/// - ensure that, across the project, global variable access is as type constrained as possible;
///
/// Note that here, a step can include sub-steps which are inlined.
///
/// Ideally this pass should happen *after* const folding so that we can be a bit
/// more certain about which branches can/will be taken or not.
/// Or is it better to do const folding after this to facilitate casts??? idk
/// TODO: const folding
///
/// To do this, we:
/// - assume that at the beginning of this pass, all variables are of indeterminate type
///   (i.e. no bits are set in the type bitset), or are already as type-constrained as possible
/// - enter a step
///   - step through each instruction
///     - if the instruction writes to a variable, create a new local variable of that type for
///       which we will now use as that variable
///     - if the instruction reads a variable, mutate that instruction's variable to be the current
///       local instance of that variable if we have one, or the local instance of that variable in
///       the outer step if we have one; otherwise just leave it
///     - if the instruction branches to a substep, then step into that step and repeat, passing in
///       the current variable map
///   - at the end of the step, for each variable:
///     - if there is a local variable for this variable in the outer step, write our current variable
///       value to that variable, and add the current variable type to the intersection of its possible
///       types
///     - otherwise, write to the global instance of that variable and add the current variable type
///       to the intersection of its possible types
///
/// Hopefully binaryen will merge all of the new uselessly created locals that this will probably
/// create, as well as removing the unnecessary global writes.
/// TODO: test to see if this is actually the case.
///
/// The difficulty here is handling branching, and how we deal with post-branch convergence; if we
/// have a branch, and then both branches converge to the same step afterwards, we want to know that
/// variable writes in that converged step will definitely happen, so we can happily make new local
/// variables rather than inflating the type of the current variable (which will happen if we can't
/// prove that the branches converge).
/// Luckily branch convergence can be declared on `control_if_else` by the `converges_to` field, and
/// when we're warped, the converging branch can be accessed normally by fallthrough anyway.
/// We just need to make sure that if convergence is explictly declared, that we return execution
/// to the outer step when we encounter a step that we know a parallel step will converge to.
/// In the future it might be worth actually splitting these up so the branches don't converge (which
/// at the moment is sometimes the case in the generated WASM anyway) so that we can determine
/// variable types with some more certainty after the branch; some investigation will be need to be
/// done to see if the trade-off of increased code size (so worse caching) for fewer branches in
/// WASM is worth it.
///
/// TODO: actually implement this
///
/// also TODO: is it possible to figure out the most likely type of a variable, so we can reorder type
/// checks and/or utilise branch hinting heuristics?
pub fn optimise_var_types(project: &Rc<IrProject>) -> HQResult<()> {
    crate::log("optimise vars");
    #[expect(
        clippy::mutable_key_type,
        reason = "implementation of Ord for Step relies on `id` field only, which is immutable"
    )]
    let mut visited_steps = BTreeSet::default();
    for thread in project.threads().try_borrow()?.iter() {
        visit_step(thread.first_step(), None, &mut visited_steps)?;
    }
    for (_, target) in project.targets().borrow().iter() {
        for (_, proc) in target.procedures()?.iter() {
            if let PartialStep::Finished(step) = proc.warped_first_step()?.clone() {
                crate::log("got warped first step for proc");
                visit_step(&step, None, &mut visited_steps)?;
            }
            if let PartialStep::Finished(step) = proc.non_warped_first_step()?.clone() {
                crate::log("got non-warped first step for proc");
                visit_step(&step, None, &mut visited_steps)?;
            }
        }
    }
    crate::log("finished optimisaing variables");
    Ok(())
}

#[expect(
    clippy::mutable_key_type,
    reason = "implementations of Eq and Ord for RcVar are independent of actual contents"
)]
#[expect(clippy::too_many_lines, reason = "shut up clippy")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "seems silly to reference an option of a reference"
)]
fn visit_step(
    step: &Rc<Step>,
    outer_variable_map: Option<&mut BTreeMap<RcVar, RcVar>>,
    visited_steps: &mut BTreeSet<Rc<Step>>,
) -> HQResult<()> {
    if !visited_steps.insert(Rc::clone(step)) {
        // we've already visited this step.
        crate::log(format!("visited step {} but it is already visited", step.id()).as_str());
        return Ok(());
    }
    crate::log(format!("visited step {}, not yet visited", step.id()).as_str());
    let mut current_variable_map = BTreeMap::new();
    let mut type_stack = vec![];
    'opcode_loop: for opcode in step.opcodes().try_borrow()?.iter() {
        let expected_inputs = opcode.acceptable_inputs()?;
        //crate::log(format!("type stack: {type_stack:?}").as_str());
        let actual_inputs: Vec<_> = type_stack
            .splice((type_stack.len() - expected_inputs.len()).., [])
            .collect();
        //crate::log(format!("inputs: {actual_inputs:?}").as_str());
        // let's just assume that input types match up.
        #[expect(clippy::wildcard_enum_match_arm, reason = "too many variants to match")]
        'opcode_block: {
            match opcode {
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var,
                    local_write: locality,
                })
                | IrOpcode::data_teevariable(DataTeevariableFields {
                    var,
                    local_read_write: locality,
                }) => {
                    crate::log("found a variable write operation");
                    if *locality.try_borrow()? {
                        crate::log("variable is already local; skipping");
                        break 'opcode_block;
                    }
                    hq_assert!(
                        !actual_inputs.is_empty(),
                        "input length should be >= 1 for variable write"
                    );
                    let input_type = actual_inputs[0];
                    let new_variable = RcVar::new(input_type, VarVal::Bool(false));
                    current_variable_map.insert(var.try_borrow()?.clone(), new_variable.clone());
                    *var.try_borrow_mut()? = new_variable;
                    *locality.try_borrow_mut()? = true;
                }
                IrOpcode::data_variable(DataVariableFields { var, local_read }) => {
                    crate::log("found a variable read operation");
                    if *local_read.try_borrow()? {
                        crate::log("variable is already local; skipping");
                        break 'opcode_block;
                    }
                    let var_to_swap = if let Some(current_var) =
                        current_variable_map.get(&var.try_borrow()?.clone())
                    {
                        crate::log("local variable instance found");
                        Some(current_var.clone())
                    } else if let Some(ref outer_var_map) = outer_variable_map
                        && let Some(outer_var) = outer_var_map.get(&var.try_borrow()?.clone())
                    {
                        crate::log("outer variable instance found");
                        Some(outer_var.clone())
                    } else {
                        None
                    };
                    if let Some(new_var) = var_to_swap {
                        *var.try_borrow_mut()? = new_var;
                        *local_read.try_borrow_mut()? = true;
                    }
                }
                IrOpcode::control_if_else(ControlIfElseFields {
                    branch_if,
                    branch_else,
                }) => {
                    crate::log("found control_if_else");
                    let if_yields = branch_if.does_yield()?;
                    // we need to make sure that if either branch yields, we visit one of those steps first,
                    // so that steps that might *not* be yielded to in the other branch are optimised as if they might
                    // be.
                    if if_yields {
                        visit_step(branch_if, Some(&mut current_variable_map), visited_steps)?;
                        visit_step(branch_else, Some(&mut current_variable_map), visited_steps)?;
                    } else {
                        visit_step(branch_else, Some(&mut current_variable_map), visited_steps)?;
                        visit_step(branch_if, Some(&mut current_variable_map), visited_steps)?;
                    }
                }
                IrOpcode::control_loop(ControlLoopFields {
                    first_condition,
                    condition,
                    body,
                    ..
                }) => {
                    crate::log("found control_loop");
                    if let Some(first_condition_step) = first_condition {
                        visit_step(
                            first_condition_step,
                            Some(&mut current_variable_map),
                            visited_steps,
                        )?;
                    }
                    visit_step(condition, Some(&mut current_variable_map), visited_steps)?;
                    visit_step(body, Some(&mut current_variable_map), visited_steps)?;
                }
                IrOpcode::hq_yield(HqYieldFields { mode }) => match mode {
                    YieldMode::Tail(_) => {
                        hq_todo!("tail-call yield in variables optimisation pass")
                    }
                    YieldMode::Inline(step) => {
                        crate::log("found inline step to visit");
                        visit_step(step, Some(&mut current_variable_map), visited_steps)?;
                    }
                    YieldMode::None => {
                        crate::log("found a yield::none, breaking");
                        break 'opcode_loop;
                    }
                    YieldMode::Schedule(step) => {
                        crate::log(
                            format!(
                                "scheduled step; strong count: {}; weak count: {}",
                                step.strong_count(),
                                step.weak_count()
                            )
                            .as_str(),
                        );
                        if let Some(rcstep) = step.upgrade()
                        //.ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Step>"))?;
                        {
                            visit_step(&rcstep, None, visited_steps)?;
                        } else {
                            crate::log("couldn't upgrade Weak<Step> in Schedule in variables optimisation pass;\
                    what's going on here?");
                        }
                    }
                },
                _ => (),
            }
        }
        if let Some(output_type) = opcode.output_type(Rc::from(actual_inputs.as_slice()))? {
            type_stack.push(output_type);
        }
    }
    // At the end of the step, we need to write the values of our localised variables to either
    // the local variables belonging to the outer scope, or to the global instance of that variable.
    // If we've written to a variable, that variable will be present as a key in `current_variable_map`;
    // it may also be present in `outer_variable_map`. So, we only need to iterate through the current
    // local variables, and then as we're doing it we can check if there's an outer equivalent.
    let final_variable_writes = current_variable_map.iter().flat_map(|(global, current)| {
        let (to_write, local) = if let Some(ref outer_var_map) = outer_variable_map
            && let Some(outer_var) = outer_var_map.get(global)
        {
            (outer_var, true)
        } else {
            (global, false)
        };
        vec![
            IrOpcode::data_variable(DataVariableFields {
                var: RefCell::new(current.clone()),
                local_read: RefCell::new(true),
            }),
            IrOpcode::data_setvariableto(DataSetvariabletoFields {
                var: RefCell::new(to_write.clone()),
                local_write: RefCell::new(local),
            }),
        ]
    });
    let post_yield = if let Some(last_op) = step.opcodes().try_borrow()?.last()
        && matches!(last_op, IrOpcode::hq_yield(_))
    {
        true
    } else {
        false
    };
    crate::log("adding outer variable writes");
    if post_yield {
        #[expect(clippy::unwrap_used, reason = "guaranteed last element")]
        let yield_op = step.opcodes_mut()?.pop().unwrap();
        step.opcodes_mut()?.extend(final_variable_writes);
        step.opcodes_mut()?.push(yield_op);
    } else {
        step.opcodes_mut()?.extend(final_variable_writes);
    }
    Ok(())
}
