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
/// Binaryen will merge all of the new uselessly created locals that this will probably
/// create, as well as removing the unnecessary global writes.
///
/// TODO: recursion
///
/// TODO: is it possible to figure out the most likely type of a variable, so we can reorder type
/// checks and/or utilise branch hinting heuristics?
pub fn optimise_var_types(project: &Rc<IrProject>) -> HQResult<()> {
    crate::log("optimise vars");
    #[expect(
        clippy::mutable_key_type,
        reason = "implementation of Ord for Step relies on `id` field only, which is immutable"
    )]
    let mut visited_steps = BTreeSet::default();
    for thread in project.threads().try_borrow()?.iter() {
        #[expect(unused_must_use, reason = "no need to use the returned iterator")]
        visit_step(
            &mut BTreeMap::new(),
            thread.first_step(),
            &None,
            &mut visited_steps,
        )?;
    }
    for (_, target) in project.targets().borrow().iter() {
        for (_, proc) in target.procedures()?.iter() {
            if let PartialStep::Finished(step) = proc.warped_first_step()?.clone() {
                crate::log("got warped first step for proc");
                #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                visit_step(&mut BTreeMap::new(), &step, &None, &mut visited_steps)?;
            }
            if let PartialStep::Finished(step) = proc.non_warped_first_step()?.clone() {
                crate::log("got non-warped first step for proc");
                #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                visit_step(&mut BTreeMap::new(), &step, &None, &mut visited_steps)?;
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
fn visit_step<'a>(
    mut current_variable_map: &'a mut BTreeMap<RcVar, RcVar>,
    step: &Rc<Step>,
    outer_variable_map: &'a Option<&'a mut BTreeMap<RcVar, RcVar>>,
    visited_steps: &mut BTreeSet<Rc<Step>>,
) -> HQResult<impl Iterator<Item = (RcVar, RcVar, bool)> + 'a> {
    if !visited_steps.insert(Rc::clone(step)) {
        // we've already visited this step.
        crate::log(format!("visited step {} but it is already visited", step.id()).as_str());
        //return Ok(vec![]); // we're not changing anything new
        return Ok(generate_changed_state(
            const { &BTreeMap::new() },
            &const { None },
        ));
    }
    crate::log(format!("visited step {}, not yet visited", step.id()).as_str());
    //let mut current_variable_map = BTreeMap::new();
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
                        #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                        visit_step(
                            &mut BTreeMap::new(),
                            branch_if,
                            &Some(&mut current_variable_map),
                            visited_steps,
                        )?;
                        #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                        visit_step(
                            &mut BTreeMap::new(),
                            branch_else,
                            &Some(&mut current_variable_map),
                            visited_steps,
                        )?;
                    } else {
                        #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                        visit_step(
                            &mut BTreeMap::new(),
                            branch_else,
                            &Some(&mut current_variable_map),
                            visited_steps,
                        )?;
                        #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                        visit_step(
                            &mut BTreeMap::new(),
                            branch_if,
                            &Some(&mut current_variable_map),
                            visited_steps,
                        )?;
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
                        #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                        visit_step(
                            &mut BTreeMap::new(),
                            first_condition_step,
                            &Some(&mut current_variable_map),
                            visited_steps,
                        )?;
                    } else {
                        #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                        visit_step(
                            &mut BTreeMap::new(),
                            condition,
                            outer_variable_map,
                            visited_steps,
                        )?;
                    }
                    /* One pass over a loop is not always enough to get the correct variable types.
                     * As an example, consider the following step:
                     * ```text
                     * set [x v] to (value of type α)
                     * repeat (pick random (0) to (42))
                     *  set [x v] to (funky function(x) (boxed value that could be type α or β)
                     * end
                     * say (x)
                     * ```
                     * where `funky function` is some sort of built-in function so no variable analysis
                     * is needed. `something`'s output type is defined as:
                     * - contains δ if either input type contains β or α;
                     * - contains β if either input type contains δ.
                     * Variable analysis pass:
                     * - create new variable `x1` with type `α`
                     * - enter loop body
                     * - create new variable `x2` with type of the output of `funky function` with
                     *   input types `α` and `α | β`
                     * - so `x2`'s type is `δ`
                     * - insert `set (x1) to (x2)` block and add `x2`'s type to `x1`
                     * - so `x1`'s type is now `α | δ`
                     * - assume that we're not generating a global variable for x because it's irrelevant
                     *   for the example
                     * The compiler thinks that `x1`'s type is `α | δ`, so the `say` block will only check
                     * for these two types. BUT, in the second iteration `x2`'s type is `β | δ`, so if we
                     * iterate more than once we end up with `x1` being `α | β | δ`, and the `say` block
                     * fails to consider that case, so we will reach an `unreachable` block.
                     * So, we need to analyse the loop body and condition repeatedly until the types of
                     * all variables stop changing. In the output `Vec` of `visit_step`, the second `RcVar`
                     * in each tuple element is the outer/global variable that has changed. So, we just
                     * need to keep track of those variables' types. When we enter these steps, we need to
                     * clone the steps themselves so they are not added to the visited steps map, and so
                     * that when they are next visited, variable reads/writes are not skipped for already
                     * being localised; when we clone, variables are not cloned so we need to extract
                     * just the variables' types before we start the next iteration. Hopefully this
                     * algorithm is deterministic, so that the same changed variables are returned every
                     * time.
                     * TODO: will this prematurely optimise inner steps?
                     */
                    let mut cloned_body = (**body).clone(false)?;
                    let mut cloned_condition = (**body).clone(false)?;
                    let prev_body_changed_state = visit_step(
                        &mut BTreeMap::new(),
                        &cloned_body,
                        &Some(&mut current_variable_map),
                        visited_steps,
                    )?
                    .map(|(_, var, _)| var)
                    .collect::<BTreeSet<_>>();
                    let prev_cond_changed_state = visit_step(
                        &mut BTreeMap::new(),
                        &cloned_condition,
                        &Some(&mut current_variable_map),
                        visited_steps,
                    )?
                    .map(|(_, var, _)| var)
                    .collect::<BTreeSet<_>>();
                    let mut changed_variables = prev_body_changed_state
                        .intersection(&prev_cond_changed_state)
                        .map(|var| (var.clone(), *var.possible_types()))
                        .collect::<BTreeMap<_, _>>();
                    'loop_convergence: loop {
                        cloned_body = (**body).clone(false)?;
                        cloned_condition = (**body).clone(false)?;
                        #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                        visit_step(
                            &mut BTreeMap::new(),
                            &cloned_body,
                            &Some(&mut current_variable_map),
                            visited_steps,
                        )?;
                        #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                        visit_step(
                            &mut BTreeMap::new(),
                            &cloned_condition,
                            &Some(&mut current_variable_map),
                            visited_steps,
                        )?;
                        let mut any_var_types_changed = false;
                        for (var, var_types) in &mut changed_variables {
                            let new_types = *var.possible_types();
                            if &new_types != var_types {
                                *var_types = new_types;
                                any_var_types_changed = true;
                            }
                        }
                        if !any_var_types_changed {
                            break 'loop_convergence;
                        }
                    }
                    // now we can optimise the real steps
                    // TODO: can we just copy the opcodes over rather than re-analysing the whole thing?
                    #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                    visit_step(
                        &mut BTreeMap::new(),
                        body,
                        &Some(&mut current_variable_map),
                        visited_steps,
                    )?;
                    #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                    visit_step(
                        &mut BTreeMap::new(),
                        condition,
                        &Some(&mut current_variable_map),
                        visited_steps,
                    )?;
                }
                IrOpcode::hq_yield(HqYieldFields { mode }) => match mode {
                    YieldMode::Tail(_) => {
                        hq_todo!("tail-call yield in variables optimisation pass")
                    }
                    YieldMode::Inline(step) => {
                        crate::log("found inline step to visit");
                        #[expect(unused_must_use, reason = "no need to use the returned iterator")]
                        visit_step(
                            &mut BTreeMap::new(),
                            step,
                            &Some(&mut current_variable_map),
                            visited_steps,
                        )?;
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
                            #[expect(
                                unused_must_use,
                                reason = "no need to use the returned iterator"
                            )]
                            visit_step(&mut BTreeMap::new(), &rcstep, &None, visited_steps)?;
                        } else {
                            crate::log("couldn't upgrade Weak<Step> in Schedule in variables optimisation pass;\
                    what's going on here?");
                        }
                        break 'opcode_loop;
                    }
                },
                _ => (),
            }
        }
        if let Some(output_type) = opcode.output_type(Rc::from(actual_inputs.as_slice()))? {
            type_stack.push(output_type);
        }
    }
    let final_state_changes = generate_changed_state(current_variable_map, outer_variable_map);
    let final_variable_writes = generate_state_backup(final_state_changes.clone());
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
    Ok(final_state_changes) //.collect())
}

/// Generate instructions to write the values of our localised variables to either the local variables
/// belonging to the outer scope, or to the global instance of that variable.
#[expect(
    clippy::mutable_key_type,
    reason = "implementations of Eq and Ord for RcVar are independent of actual contents"
)]
fn generate_changed_state<'a>(
    current_variable_map: &'a BTreeMap<RcVar, RcVar>,
    outer_variable_map: &'a Option<&mut BTreeMap<RcVar, RcVar>>,
) -> impl Iterator<Item = (RcVar, RcVar, bool)> + Clone + 'a {
    // If we've written to a variable, that variable will be present as a key in `current_variable_map`;
    // it may also be present in `outer_variable_map`. So, we only need to iterate through the current
    // local variables, and then as we're doing it we can check if there's an outer equivalent.
    current_variable_map.iter().map(move |(global, current)| {
        let (to_write, local) = if let Some(ref outer_var_map) = outer_variable_map
            && let Some(outer_var) = outer_var_map.get(global)
        {
            (outer_var, true)
        } else {
            (global, false)
        };
        (current.clone(), to_write.clone(), local)
    })
}

fn generate_state_backup(
    changed_state: impl Iterator<Item = (RcVar, RcVar, bool)>,
) -> Vec<IrOpcode> {
    changed_state
        .flat_map(|(current, to_write, local)| {
            to_write.add_type(*current.possible_types());
            vec![
                IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(current),
                    local_read: RefCell::new(true),
                }),
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(to_write),
                    local_write: RefCell::new(local),
                }),
            ]
        })
        .collect()
}
