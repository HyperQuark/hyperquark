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
    /// the variable was accessed, when it previously wasn't live
    BecameLive,
    /// the variable died by virtue of encountering a `first_write` access
    FirstWriteDead,
}

#[derive(Clone, Debug)]
struct LiveRangeEvent {
    // we record the index so we can record events out of order, for example
    // if we need to extend a liveness range to encompass the entirety of a
    // loop, it's easiest not to have to reorder a big list of events.
    // The index is arbitrary as long as it's ordered (increasing in the order
    // of traversal).
    index: usize,
    kind: LivenessEvent,
    var: RcVar,
}

fn record_event(events: &mut Vec<LiveRangeEvent>, var: &RcVar, kind: LivenessEvent) {
    events.push(LiveRangeEvent {
        index: events.len(),
        kind,
        var: var.clone(),
    });
}

fn record_event_at(
    events: &mut Vec<LiveRangeEvent>,
    index: usize,
    var: &RcVar,
    kind: LivenessEvent,
) {
    events.push(LiveRangeEvent {
        index,
        kind,
        var: var.clone(),
    });
}

#[derive(Clone, Debug)]
struct LivenessRange {
    var: RcVar,
    start: usize,
    end: usize,
}

#[expect(clippy::mutable_key_type, reason = "hash depends only on immutable id")]
fn build_merges_from_events(events: &[LiveRangeEvent]) -> BTreeMap<RcVar, RcVar> {
    // Record the maximal liveness range for each variable
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

    // Build liveness ranges for variables that do actually die at some point, and gather
    // in buckets of the same type
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
        // sort this set of variables that have the same type by their liveness ranges,
        // first by alive time and then by death time
        ranges.sort_by(|a, b| {
            a.start
                .cmp(&b.start)
                .then(a.end.cmp(&b.end))
                .then(a.var.cmp(&b.var))
        });

        // the last merged variables in each merge group
        let mut joined: BinaryHeap<Reverse<(usize, RcVar)>> = BinaryHeap::new();

        for range in &*ranges {
            let mut extracted = BinaryHeap::new();
            // pop the variables with the earliest finish times to find the optimal group
            // to join, or create a new merge group if none are available
            // TODO: is there any more information we can store in order to make this step
            // more efficient? Is it better to store merge groups in a BST and do binary search?
            // This current approach doesn't require sorting at insertion time but the trade off
            // is then linear search.
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

/// Traverses backwards through the opcodes of a step to build a sequence of
/// liveness events in `events`.
#[expect(clippy::mutable_key_type, reason = "hash depends only on immutable id")]
fn build_liveness_events<S>(
    step: S,
    live_variables: &mut BTreeSet<RcVar>,
    first_write_kills: &mut BTreeSet<RcVar>,
    events: &mut Vec<LiveRangeEvent>,
    last_action_reads: &mut Option<&mut BTreeSet<RcVar>>,
    loop_writes: &mut Option<&mut BTreeSet<RcVar>>,
) -> HQResult<()>
where
    S: Deref<Target = RefCell<Step>>,
{
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
                    if let Some(reads) = last_action_reads.as_mut() {
                        reads.remove(&real_var);
                    }
                    if let Some(writes) = loop_writes.as_deref_mut() {
                        writes.insert(real_var.clone());
                    }
                    live_variables.remove(&real_var);
                    first_write_kills.insert(real_var.clone());
                    record_event(events, &real_var, LivenessEvent::FirstWriteDead);
                }
            }
            IrOpcode::data_variable(DataVariableFields {
                var,
                local_read: local,
            }) => {
                if *local.try_borrow()? {
                    let real_var = var.try_borrow()?.clone();
                    if let Some(reads) = last_action_reads.as_mut() {
                        reads.insert(real_var.clone());
                    }
                    add_live_variable(live_variables, events, &real_var);
                }
            }
            IrOpcode::data_teevariable(DataTeevariableFields {
                var,
                local_read_write: local,
            }) => {
                if *local.try_borrow()? {
                    let real_var = var.try_borrow()?.clone();
                    if let Some(reads) = last_action_reads.as_mut() {
                        reads.remove(&real_var);
                    }
                    if let Some(writes) = loop_writes.as_deref_mut() {
                        writes.insert(real_var.clone());
                    }
                    add_live_variable(live_variables, events, &real_var);
                }
            }
            IrOpcode::data_setvariableto(DataSetvariabletoFields {
                var,
                local_write: local,
                first_write,
            }) if !*first_write.try_borrow()? => {
                if *local.try_borrow()? {
                    let real_var = var.try_borrow()?.clone();
                    if let Some(reads) = last_action_reads.as_mut() {
                        reads.remove(&real_var);
                    }
                    if let Some(writes) = loop_writes.as_deref_mut() {
                        writes.insert(real_var.clone());
                    }
                    add_live_variable(live_variables, events, &real_var);
                }
            }
            IrOpcode::hq_yield(HqYieldFields {
                mode: YieldMode::Inline(inline_step),
            }) => {
                build_liveness_events(
                    Rc::clone(inline_step),
                    live_variables,
                    first_write_kills,
                    events,
                    last_action_reads,
                    loop_writes,
                )?;
            }
            IrOpcode::control_if_else(ControlIfElseFields {
                branch_if,
                branch_else,
            }) => {
                let mut if_live = live_variables.clone();
                let mut if_kills = first_write_kills.clone();
                let mut if_last = last_action_reads.as_deref().cloned();
                let mut if_writes = loop_writes.as_deref().cloned();
                build_liveness_events(
                    Rc::clone(branch_if),
                    &mut if_live,
                    &mut if_kills,
                    events,
                    &mut if_last.as_mut(),
                    &mut if_writes.as_mut(),
                )?;
                let mut else_live = live_variables.clone();
                let mut else_kills = first_write_kills.clone();
                let mut else_last = last_action_reads.as_deref().cloned();
                let mut else_writes = loop_writes.as_deref().cloned();
                build_liveness_events(
                    Rc::clone(branch_else),
                    &mut else_live,
                    &mut else_kills,
                    events,
                    &mut else_last.as_mut(),
                    &mut else_writes.as_mut(),
                )?;
                #[expect(
                    clippy::unwrap_used,
                    reason = "if `last_action_reads` is `Some` then so must be `if_last` and \
                              `else_last` as they are cloned from it"
                )]
                if let Some(reads) = last_action_reads.as_deref_mut() {
                    *reads = if_last
                        .unwrap()
                        .union(else_last.as_ref().unwrap())
                        .cloned()
                        .collect();
                }
                #[expect(clippy::unwrap_used, reason = "same as above")]
                if let Some(writes) = loop_writes.as_deref_mut() {
                    *writes = if_writes
                        .unwrap()
                        .union(else_writes.as_ref().unwrap())
                        .cloned()
                        .collect();
                }
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
                let loop_start_index = events.len();
                let mut loop_live = live_variables.clone();

                // Track reads and writes discovered WITHIN this loop
                let mut loop_last_action_reads = BTreeSet::new();
                let mut inner_loop_writes = BTreeSet::new();

                if first_condition.is_none() {
                    build_liveness_events(
                        Rc::clone(condition),
                        &mut loop_live,
                        first_write_kills,
                        events,
                        &mut Some(&mut loop_last_action_reads),
                        &mut Some(&mut inner_loop_writes),
                    )?;
                }
                build_liveness_events(
                    Rc::clone(body),
                    &mut loop_live,
                    first_write_kills,
                    events,
                    &mut Some(&mut loop_last_action_reads),
                    &mut Some(&mut inner_loop_writes),
                )?;
                pre_body
                    .as_ref()
                    .map(|pre_body_step| {
                        build_liveness_events(
                            Rc::clone(pre_body_step),
                            &mut loop_live,
                            first_write_kills,
                            events,
                            &mut Some(&mut loop_last_action_reads),
                            &mut Some(&mut inner_loop_writes),
                        )
                    })
                    .transpose()?;

                if let Some(first_condition_step) = first_condition {
                    build_liveness_events(
                        Rc::clone(first_condition_step),
                        &mut loop_live,
                        first_write_kills,
                        events,
                        &mut Some(&mut loop_last_action_reads),
                        &mut Some(&mut inner_loop_writes),
                    )?;
                } else {
                    build_liveness_events(
                        Rc::clone(condition),
                        &mut loop_live,
                        first_write_kills,
                        events,
                        &mut Some(&mut loop_last_action_reads),
                        &mut Some(&mut inner_loop_writes),
                    )?;
                }

                for var in &loop_last_action_reads {
                    // if the variable is read before anything else in the loop, and if it wasn't
                    // live before the loop, extend its liveness range to the forward-direction end
                    // of the loop (which is explored first. confusing eh)
                    if !live_variables.contains(var) {
                        record_event_at(events, loop_start_index, var, LivenessEvent::BecameLive);
                    }
                    // there's no need to extend the liveness range to the start of the loop, because if
                    // the last observed access was a read then it is impossible for a first_write to have
                    // been observed, so the variable must still be live.
                }

                *live_variables = loop_live;

                if let Some(reads) = last_action_reads.as_deref_mut() {
                    *reads = reads
                        // remove any variables that have been written to
                        .difference(&loop_writes.as_deref().cloned().unwrap_or_default())
                        // and readd any that still have their last action being a read
                        .chain(&loop_last_action_reads)
                        .cloned()
                        .collect();
                }
                if let Some(writes) = last_action_reads.as_deref_mut() {
                    *writes = writes.union(&inner_loop_writes).cloned().collect();
                }
            }
            _ => (),
        }
    }
    Ok(())
}

/// Replaces variable accesses in a step with a target merge
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
/// This merges variables that have exactly the same type; depending on the "type convergence"
/// flag, this might mean the same base types, or all variables.
///
/// This bases its liveness analysis on information provided in `data_setvariableto`
/// instructions, so that it can run linearly-ish in the number of instructions. Therefore
/// it does need to be run after SSA.
pub fn merge_variables(proj: &Rc<IrProject>, _ssa_token: SSAToken) -> HQResult<()> {
    for step in proj.steps().borrow().iter() {
        let mut events = Vec::new();
        build_liveness_events(
            step,
            &mut BTreeSet::new(),
            &mut BTreeSet::new(),
            &mut events,
            &mut None,
            &mut None,
        )?;
        #[expect(clippy::mutable_key_type, reason = "hash depends only on immutable id")]
        let merged = build_merges_from_events(&events);
        merge_variables_in_step(step, &merged)?;
    }

    Ok(())
}
