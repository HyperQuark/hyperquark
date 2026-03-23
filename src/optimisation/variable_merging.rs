use alloc::collections::BinaryHeap;
use core::cmp::Reverse;
use core::ops::Deref;

use crate::instructions::{
    ControlIfElseFields, ControlLoopFields, DataSetvariabletoFields, DataTeevariableFields,
    DataVariableFields, HqYieldFields, IrOpcode, YieldMode,
};
use crate::ir::{IrProject, RcVar, Step, Type as IrType};
use crate::optimisation::SSAToken;
use crate::prelude::*;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum LivenessEvent {
    BecameLive,
    FirstWriteDead,
}

#[derive(Clone, Debug)]
struct LiveRangeEvent {
    index: usize,
    kind: LivenessEvent,
    var: RcVar,
}

#[derive(Clone, Debug)]
struct LivenessRange {
    var: RcVar,
    start: usize,
    end: usize,
}

fn record_event(events: &mut Vec<LiveRangeEvent>, var: &RcVar, kind: LivenessEvent) {
    events.push(LiveRangeEvent {
        index: events.len(),
        kind,
        var: var.clone(),
    });
}

#[expect(clippy::mutable_key_type, reason = "hash depends only on immutable id")]
fn build_optimal_merges_from_events(events: &[LiveRangeEvent]) -> BTreeMap<RcVar, RcVar> {
    let mut liveness_ranges_builder: BTreeMap<RcVar, (Option<usize>, Option<usize>)> =
        BTreeMap::new();
    for event in events {
        let (live_time, dead_time) = liveness_ranges_builder
            .entry(event.var.clone())
            .or_insert_with(|| (None, None));
        match event.kind {
            LivenessEvent::BecameLive => {
                *live_time = Some(live_time.map_or(event.index, |old| old.min(event.index)));
            }
            LivenessEvent::FirstWriteDead => {
                *dead_time = Some(dead_time.map_or(event.index, |old| old.max(event.index)));
            }
        }
    }

    let mut liveness_ranges: BTreeMap<IrType, Vec<LivenessRange>> = BTreeMap::new();
    for (var, (birth, death)) in liveness_ranges_builder {
        let Some(dead_index) = death else {
            continue;
        };
        let live_index = birth.unwrap_or(dead_index);
        let (start, end) = if live_index <= dead_index {
            (live_index, dead_index)
        } else {
            (dead_index, live_index)
        };
        liveness_ranges
            .entry(*var.possible_types())
            .or_default()
            .push(LivenessRange {
                var: var.clone(),
                start,
                end,
            });
    }

    #[expect(clippy::mutable_key_type, reason = "hash depends only on immutable id")]
    let mut merged = BTreeMap::new();
    for ranges in liveness_ranges.values_mut() {
        ranges.sort_by(|a, b| {
            a.start
                .cmp(&b.start)
                .then(a.end.cmp(&b.end))
                .then(a.var.cmp(&b.var))
        });

        let mut joined: BinaryHeap<Reverse<(usize, RcVar)>> = BinaryHeap::new();
        for range in &*ranges {
            let mut extracted = BinaryHeap::new();
            while let Some(best_candidate) = joined.pop() {
                if best_candidate.0.0 < range.start {
                    merged.insert(
                        range.var.clone(),
                        merged
                            .get(&best_candidate.0.1)
                            .unwrap_or(&best_candidate.0.1)
                            .clone(),
                    );
                    break;
                }
                extracted.push(best_candidate);
            }
            joined.append(&mut extracted);
            joined.push(Reverse((range.end, range.var.clone())));
        }
    }

    merged
}

#[expect(clippy::mutable_key_type, reason = "hash depends only on immutable id")]
fn add_live_variable(
    live_variables: &mut BTreeSet<RcVar>,
    events: &mut Vec<LiveRangeEvent>,
    var: &RcVar,
) {
    let new = live_variables.insert(var.clone());
    if new {
        record_event(events, var, LivenessEvent::BecameLive);
    }
}

#[expect(clippy::mutable_key_type, reason = "hash depends only on immutable id")]
fn find_vars_to_merge<S>(
    step: S,
    live_variables: &mut BTreeSet<RcVar>,
    first_write_kills: &mut BTreeSet<RcVar>,
    events: &mut Vec<LiveRangeEvent>,
) -> HQResult<()>
where
    S: Deref<Target = RefCell<Step>>,
{
    crate::log!("merging vars for step {}", step.try_borrow()?.id());
    for block in step.try_borrow()?.opcodes().iter().rev() {
        #[expect(
            clippy::wildcard_enum_match_arm,
            reason = "too many variants to match individually"
        )]
        match block {
            IrOpcode::data_setvariableto(DataSetvariabletoFields {
                var,
                local_write,
                first_write,
            }) if *first_write.try_borrow()? => {
                if *local_write.try_borrow()? {
                    let real_var = var.try_borrow()?.clone();
                    live_variables.remove(&real_var);
                    first_write_kills.insert(real_var.clone());
                    record_event(events, &real_var, LivenessEvent::FirstWriteDead);
                }
            }
            IrOpcode::data_variable(DataVariableFields {
                var,
                local_read: local,
            })
            | IrOpcode::data_teevariable(DataTeevariableFields {
                var,
                local_read_write: local,
            }) => {
                if *local.try_borrow()? {
                    add_live_variable(live_variables, events, &*var.try_borrow()?);
                }
            }
            IrOpcode::data_setvariableto(DataSetvariabletoFields {
                var,
                local_write: local,
                first_write,
            }) if !*first_write.try_borrow()? => {
                if *local.try_borrow()? {
                    add_live_variable(live_variables, events, &*var.try_borrow()?);
                }
            }
            IrOpcode::hq_yield(HqYieldFields {
                mode: YieldMode::Inline(inline_step),
            }) => {
                find_vars_to_merge(
                    Rc::clone(inline_step),
                    live_variables,
                    first_write_kills,
                    events,
                )?;
            }
            IrOpcode::control_if_else(ControlIfElseFields {
                branch_if,
                branch_else,
            }) => {
                let mut if_live = live_variables.clone();
                let mut if_kills = first_write_kills.clone();
                find_vars_to_merge(Rc::clone(branch_if), &mut if_live, &mut if_kills, events)?;
                let mut else_live = live_variables.clone();
                let mut else_kills = first_write_kills.clone();
                find_vars_to_merge(
                    Rc::clone(branch_else),
                    &mut else_live,
                    &mut else_kills,
                    events,
                )?;

                // if a variable died in either branch, it can't be accessed past that branch
                // so must be dead after both branches
                *first_write_kills = if_kills.union(&else_kills).cloned().collect();

                // a variable is live past the branch if it is live in either branch AND
                // wasn't found dead in either branch. We can't go off of being live in
                // both branches because it might only exist in one branch
                *live_variables = if_live.union(&else_live).cloned().collect();
                live_variables.retain(|var| !first_write_kills.contains(var));
            }
            IrOpcode::control_loop(ControlLoopFields {
                body,
                pre_body,
                first_condition,
                condition,
                ..
            }) => {
                if first_condition.is_some() {
                    find_vars_to_merge(
                        Rc::clone(condition),
                        live_variables,
                        first_write_kills,
                        events,
                    )?;
                }
                find_vars_to_merge(Rc::clone(body), live_variables, first_write_kills, events)?;
                pre_body
                    .as_ref()
                    .map(|pre_body_step| {
                        find_vars_to_merge(
                            Rc::clone(pre_body_step),
                            live_variables,
                            first_write_kills,
                            events,
                        )
                    })
                    .transpose()?;
                if let Some(first_condition_step) = first_condition {
                    find_vars_to_merge(
                        Rc::clone(first_condition_step),
                        live_variables,
                        first_write_kills,
                        events,
                    )?;
                } else {
                    find_vars_to_merge(
                        Rc::clone(condition),
                        live_variables,
                        first_write_kills,
                        events,
                    )?;
                }
            }
            _ => (),
        }
    }
    // crate::log!(
    //     "{} killed variables, {} live",
    //     first_write_kills.len(),
    //     live_variables.len(),
    // );
    Ok(())
}

#[expect(clippy::mutable_key_type, reason = "hash depends only on immutable id")]
pub fn merge_variables_in_step<S>(step: S, merges: &BTreeMap<RcVar, RcVar>) -> HQResult<()>
where
    S: Deref<Target = RefCell<Step>>,
{
    for opcode in step.try_borrow()?.opcodes() {
        if let IrOpcode::data_variable(DataVariableFields { var, .. })
        | IrOpcode::data_setvariableto(DataSetvariabletoFields { var, .. })
        | IrOpcode::data_teevariable(DataTeevariableFields { var, .. }) = opcode
        {
            let real_var = var.try_borrow()?.clone();
            if let Some(merge) = merges.get(&real_var).cloned() {
                *var.try_borrow_mut()? = merge;
            }
        }

        if let Some(inline_steps) = opcode.inline_steps(false) {
            for inline_step in inline_steps {
                merge_variables_in_step(inline_step, merges)?;
            }
        }
    }
    Ok(())
}

/// Merges variables that have the same type, only if their usage does not overlap.
///
/// This is important because we can sometimes produce far too many locals for the
/// browser to be able to compile the WASM module, and as Binaryen's coalesce-locals
/// pass is "non-linear in the number of locals", WASM optimisation can take an
/// unacceptably long time.
///
/// At the moment, this only merges variables that have exactly the same type, not just
/// the same basic type, to preserve the maximal amount of information.
///
/// This also bases its liveness analysis on information provided in `data_setvariableto`
/// instructions, so that it can run linearly-ish in the number of instructions. Therefore
/// it does need to be run after SSA.
///
/// TODO: make this configurable? Or find some other way to indicate the type of a variable
/// at a specific point in time.
pub fn merge_variables(proj: &Rc<IrProject>, _ssa_token: SSAToken) -> HQResult<()> {
    for step in proj.steps().borrow().iter() {
        let mut events = Vec::new();
        find_vars_to_merge(
            step,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut events,
        )?;
        #[expect(clippy::mutable_key_type, reason = "hash depends only on immutable id")]
        let merged = build_optimal_merges_from_events(&events);
        merge_variables_in_step(step, &merged)?;
        crate::log!(
            "merging {} variables into {}",
            merged.len(),
            merged.values().collect::<BTreeSet<_>>().len(),
        );
    }

    Ok(())
}
