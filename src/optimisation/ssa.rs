//! Carry out SSA (static single assignment) transformation on the project, and fix up
//! some variable shennanigans.
//!
//! The goals are:
//! - Split variables so they are only ever written to once, and are read from the most recent
//!   instance of the variable
//! - Change variable reads inside procedures to argument access, if that variable hasn't been
//!   written to inside that procedure yet
//! - Assign the narrowest type possible to each variable
//! - Reinsert casts to account for newly assigned variable types.
//! - Box procedure returns if needed
//!
//! This will create a lot of unnecessary variables; this is ok, as wasm-opt does local merging
//! and should remove unnecessary tees etc. But this might be an area in which performance could
//! be improved, if we can reduce the number of variables we need to track on our end, which
//! will also improve wasm-opt's runtime.
//!
//! ## Rough description of algorithm
//!
//! This does not correspond exactly to the implementation, but hopefully explains the intent.
//!
//! ```pseudocode
//! function split_to_SSA (step, ssa_map = {}, mut additional_opcodes):
//!   for each opcode in step:
//!     if opcode is a variable read and is not already local:
//!       if opcode not in ssa_map:
//!         if step context is in procedure:
//!           replace opcode with the appropriate argument access
//!       else:
//!         replace opcode with local variable read to corresponding variable in ssa_map
//!     if opcode is a variable write/tee to var and is not already local:
//!       new_var := new variable
//!       ssa_map[var] = new_var
//!       replace opcode with local variable write/tee to new_var
//!     if opcode yields inline to other_steps (e.g. by yield_inline, loop or if/else):
//!       if opcode is loop:
//!         for each globally accessible variable:
//!           write to a new SSA with the current value of the variable
//!           # the needless SSAs here will be removed by wasm-opt.
//!           # TODO: can we keep track of which variables are accessed from inside each step, to simplify this step?
//!       lower_ssa_maps := []
//!       for each lower_step in other_steps:
//!         append (split_to_SSA(other step, ssa_map, additional_opcodes)) to lower_ssa_maps
//!       consolidate lower_ssa_maps into lower_ssa_map of variable : [variable]
//!       for each (global, locals) in lower_ssa_map:
//!         if locals.length == 1:
//!           ssa_map[global] := locals[0]
//!         else:
//!           new_var := new variable
//!           for each i, lower_step in enumerate(other_steps):
//!             append (local variable write to new_var from locals[i])
//!                 to additional_opcodes[lower_step]
//!           ssa_map[global] = new_var
//!     if opcode yields non-inline to other_step:
//!       split_to_SSA(other_step, {}, additional_steps)
//!   if (step is not inlined and step context is not in procedure) or (last opcode was a non-inline yield):
//!     for (global, local) in ssa_map:
//!       append (local variable read of local) to additional_opcodes[step]
//!       append (global variable write of global) to additional_opcodes[step]
//!     return {}
//!   if step context is in procedure:
//!     return {}
//!   return ssa_map
//!
//! additional_opcodes = {}
//!
//! for step in top level steps:
//!   split_to_SSA (step, {}, additional_opcodes)
//!
//! for (step, extra_opcodes) in additional_opcodes:
//!   concat extra_opcodes to step.opcodes
//! ```
//!
//! Alongside this SSA splitting, we also need to construct a graph to reason about what types
//! might be on the stack at any one point. This is simpler so won't be explained here.
//!
//! A note on naming: I call this SSA, but it is not strictly true. In SSA, each variable is assigned to
//! exactly once, and at a dominance frontier, SSAs from preceding blocks are merged into one using a 'phi node'
//! or 'phi function', which picks the correct variable to read based on which branch was just taken. Carrying
//! around additional information about which block was just taken isn't something that I want to do, so our
//! 'phi function' actually inserts the relevant assignment into the preceding blocks directly. This does mean
//! that some variables can be written to from different blocks, especially where loops are concerned (as the
//! output SSA needs to be written to both before the loop and at the end of each iteration). This isn't actually
//! a problem for us, as we're not using SSA to carry out liveness analysis - we use it to minimise casts and
//! maximimise the type information known at compile time.

#![expect(
    clippy::mutable_key_type,
    reason = "implementations of Eq and Ord for RcVar and Step are independent of actual contents"
)]

mod box_proc_returns;
mod type_convergence;
mod var_graph;

use alloc::collections::btree_map::Entry;
use core::convert::identity;
use core::marker::PhantomData;

use box_proc_returns::box_proc_returns;
use type_convergence::iterate_graphs;
use var_graph::{MaybeGraph, VarGraph, VariableMaps};

use crate::ir::{IrProject, PartialStep, Proc, RcVar, StepIndex, insert_casts};
use crate::prelude::*;
use crate::wasm::flags::{Switch, VarTypeConvergence};

fn visit_step_recursively(
    step_index: StepIndex,
    project: &Rc<IrProject>,
    graphs: &mut BTreeMap<Box<str>, MaybeGraph>,
    proc_args: &BTreeMap<RcVar, (usize, RcVar)>,
    do_ssa: bool,
) -> HQResult<()> {
    let steps = project.steps().try_borrow()?;
    let next_steps = &mut vec![step_index];
    while let Some(next_step_index) = next_steps.pop() {
        let next_step = steps
            .get(next_step_index.0)
            .ok_or_else(|| make_hq_bug!("step index out of bounds"))?;
        let entry = graphs.entry(next_step.try_borrow()?.id().into());
        // it can happen that the same step can be visited from multiple different points,
        // especially where if/elses are involved. If this is the case, we don't want to revisit
        // that step, as it may have been modified during the last pass, in which case another
        // visit will apply the same modifications again, and various things will be duplicated
        // and will cause problems. For that reason, we only visit this step if the current entry
        // for that step is vacant.
        if let Entry::Vacant(vacant_entry) = entry {
            let mut graph = VarGraph::new();
            vacant_entry.insert(MaybeGraph::Started);
            graph.visit_step(
                next_step,
                &mut VariableMaps::new_with_proc_args(proc_args),
                graphs,
                next_steps,
                do_ssa,
            )?;
            graphs.insert(
                next_step.try_borrow()?.id().into(),
                MaybeGraph::Finished(graph),
            );
        }
    }
    Ok(())
}

fn visit_procedure(
    proc: &Rc<Proc>,
    graphs: &mut BTreeMap<Box<str>, MaybeGraph>,
    project: &Rc<IrProject>,
    do_ssa: bool,
) -> HQResult<()> {
    if let Some(warped_specific_proc) = &*proc.warped_specific_proc()
        && let PartialStep::Finished(step_index) = warped_specific_proc.first_step()?.clone()
    {
        let steps = project.steps().try_borrow()?;
        let step = steps
            .get(step_index.0)
            .ok_or_else(|| make_hq_bug!("step index out of bounds"))?;
        let globally_scoped_variables: Box<[_]> =
            step.try_borrow()?.globally_scoped_variables()?.collect();
        let arg_vars_drop =
            warped_specific_proc.arg_vars().try_borrow()?.len() - globally_scoped_variables.len();
        visit_step_recursively(
            step_index,
            project,
            graphs,
            &globally_scoped_variables
                .into_iter()
                .zip(
                    warped_specific_proc
                        .arg_vars()
                        .try_borrow()?
                        .iter()
                        .cloned()
                        .enumerate()
                        .dropping(arg_vars_drop),
                )
                .collect(),
            do_ssa,
        )?;
    }
    if let Some(nonwarped_specific_proc) = &*proc.nonwarped_specific_proc()
        && let PartialStep::Finished(step_index) = nonwarped_specific_proc.first_step()?.clone()
    {
        visit_step_recursively(step_index, project, graphs, &BTreeMap::new(), do_ssa)?;
    }
    Ok(())
}

/// Splits variables at every write site, and ensures that outer/global variables are then written
/// to at the end of a step. This also constructs variable type graphs for each step which can then
/// be analyzed to determine the best types for variables.
fn split_variables_and_make_graphs(
    project: &Rc<IrProject>,
    do_ssa: bool,
) -> HQResult<BTreeMap<Box<str>, MaybeGraph>> {
    let mut graphs = BTreeMap::new();
    for (_, target) in project.targets().borrow().iter() {
        for (_, proc) in target.procedures()?.iter() {
            visit_procedure(proc, &mut graphs, project, do_ssa)?;
        }
    }
    for thread in project.threads().try_borrow()?.iter() {
        let step_index = thread.first_step();
        visit_step_recursively(step_index, project, &mut graphs, &BTreeMap::new(), do_ssa)?;
    }
    Ok(graphs)
}

/// A token type that cannot be instantiated from anywhere else (since the field is private)
/// - used as proof that we've carried out these optimisations.
#[derive(Copy, Clone)]
pub struct SSAToken(PhantomData<()>);

pub fn optimise_variables(
    project: &Rc<IrProject>,
    type_convergence: VarTypeConvergence,
    do_ssa: Switch,
) -> HQResult<SSAToken> {
    let induced_do_ssa = do_ssa == Switch::On && type_convergence != VarTypeConvergence::Any;
    let maybe_graphs = split_variables_and_make_graphs(project, induced_do_ssa)?;
    let graphs = maybe_graphs
        .iter()
        .map(|(step, graph)| {
            Ok(match graph {
                MaybeGraph::Started => hq_bug!("found unfinished and non-inlined var graph"),
                MaybeGraph::Inlined => None,
                MaybeGraph::Finished(finished_graph) => Some((step, finished_graph)),
            })
        })
        .filter_map_ok(identity)
        .collect::<HQResult<BTreeMap<_, _>>>()?;
    iterate_graphs(
        &graphs.iter().map(|(s, g)| (*g, (*s).clone())),
        type_convergence,
    )?;
    for step in project.steps().try_borrow()?.iter() {
        insert_casts(step.try_borrow_mut()?.opcodes_mut(), false, true)?;
    }

    // we might have made some procedure return types boxed, so we now need to go through and box the
    // appropriate variable getters, now that we know *which* variables are boxed.
    for target in project.targets().borrow().values() {
        for procedure in target.procedures()?.values() {
            if let Some(warped_specific_proc) = &*procedure.warped_specific_proc()
                && let PartialStep::Finished(step) = &*warped_specific_proc.first_step()?
            {
                box_proc_returns(
                    project
                        .steps()
                        .try_borrow()?
                        .get(step.0)
                        .ok_or_else(|| make_hq_bug!("step index out of bounds"))?,
                    &(*warped_specific_proc.return_vars())
                        .borrow()
                        .iter()
                        .rev()
                        .collect::<Box<[_]>>(),
                    &mut BTreeSet::new(),
                )?;
            }
        }
    }

    Ok(SSAToken(PhantomData))
}
