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
//! or 'phi function', why picks the correct variable to read based on which branch was just taken. Carrying
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

use crate::instructions::{
    ControlIfElseFields, ControlLoopFields, DataAddtolistFields, DataItemoflistFields,
    DataReplaceitemoflistFields, DataSetvariabletoFields, DataTeevariableFields,
    DataVariableFields, HqYieldFields, IrOpcode, ProceduresArgumentFields,
    ProceduresCallWarpFields, YieldMode,
};
use crate::ir::{
    IrProject, PartialStep, Proc, RcList, RcVar, ReturnType, Step, Type as IrType, insert_casts,
};
use crate::prelude::*;
use crate::sb3::VarVal;

use alloc::collections::btree_map::Entry;
use core::convert::identity;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::mem;
// use petgraph::dot::Dot;

use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::{Incoming as EdgeIn, Outgoing as EdgeOut, stable_graph::StableDiGraph};

#[derive(Clone, Debug)]
enum StackElement {
    Var(RcVar),
    List(RcList),
    // for things where we can guarantee the output type. e.g. for constants (int, float, string)
    Type(IrType),
    /// this shouldn't be anything that branches or mutates anything in any way,
    /// i.e. not loop, if/else, yield, set variable, etc
    Opcode(IrOpcode),
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
enum VarOrList {
    Var(RcVar),
    List(RcList),
}

impl VarOrList {
    fn possible_types(&self) -> core::cell::Ref<'_, IrType> {
        match self {
            Self::Var(var) => var.possible_types(),
            Self::List(list) => list.possible_types(),
        }
    }

    fn add_type(&self, ty: IrType) {
        match self {
            Self::Var(var) => var.add_type(ty),
            Self::List(list) => list.add_type(ty),
        }
    }
}

/// Is this edge a backlink or not?
///
/// We could calculate this dynamically during the reduction stage but seeing as we can know this
/// when constructing the graph, we might as well do it now.
#[derive(Copy, Clone, Debug, PartialEq)]
enum EdgeType {
    Forward,
    BackLink,
}

type NodeWeight = Option<(Vec<VarOrList>, Vec<StackElement>)>;

#[derive(Debug)]
struct BaseVarGraph {
    graph: RefCell<StableDiGraph<NodeWeight, EdgeType>>,
    exit_node: RefCell<NodeIndex>,
}

#[derive(Clone, Debug)]
struct VarGraph(Rc<BaseVarGraph>);

impl Hash for VarGraph {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.0).hash(state);
    }
}

enum MaybeGraph {
    Started,
    Inlined,
    Finished(VarGraph),
}

#[derive(Clone)]
struct VariableMaps<'a> {
    /// global variable -> local variable
    pub ssa: BTreeMap<RcVar, RcVar>,
    /// global variable -> (arg index, arg variable)
    pub proc_args: &'a BTreeMap<RcVar, (usize, RcVar)>,
}

impl<'a> VariableMaps<'a> {
    const fn new_with_proc_args(proc_args: &'a BTreeMap<RcVar, (usize, RcVar)>) -> Self {
        Self {
            ssa: BTreeMap::new(),
            proc_args,
        }
    }

    fn use_current_ssa_for_global<S, A, G, T>(
        &self,
        global_var: &RcVar,
        if_ssa: S,
        if_arg: A,
        if_global: G,
    ) -> T
    where
        S: FnOnce(&RcVar) -> T,
        A: FnOnce(usize, &RcVar) -> T,
        G: FnOnce() -> T,
    {
        self.ssa.get(global_var).map_or_else(
            || {
                if let Some((arg_index, arg_var)) = self.proc_args.get(global_var) {
                    if_arg(*arg_index, arg_var)
                } else {
                    if_global()
                }
            },
            if_ssa,
        )
    }
}

impl VarGraph {
    pub fn new() -> Self {
        let mut graph = StableDiGraph::new();
        let exit_node = graph.add_node(None);
        Self(Rc::new(BaseVarGraph {
            graph: RefCell::new(graph),
            exit_node: RefCell::new(exit_node),
        }))
    }

    fn graph(&self) -> &RefCell<StableDiGraph<NodeWeight, EdgeType>> {
        &self.0.graph
    }

    fn exit_node(&self) -> &RefCell<NodeIndex> {
        &self.0.exit_node
    }

    fn add_node(&self, weight: NodeWeight) -> NodeIndex {
        self.graph().borrow_mut().add_node(weight)
    }

    fn add_edge(&self, a: NodeIndex, b: NodeIndex, weight: EdgeType) -> EdgeIndex {
        self.graph().borrow_mut().add_edge(a, b, weight)
    }

    /// Visits a step to split its variables and construct a variable graph for it.
    #[expect(clippy::too_many_lines, reason = "difficult to split")]
    fn visit_step(
        &mut self,
        step: &Rc<Step>,
        variable_maps: &mut VariableMaps<'_>,
        graphs: &mut BTreeMap<Rc<Step>, MaybeGraph>,
        type_stack: &mut Vec<StackElement>,
        next_steps: &mut Vec<Rc<Step>>,
    ) -> HQResult<()> {
        // crate::log!(
        //     "searched in graphs for step {}, got {}",
        //     step.id(),
        //     match graphs.get(step) {
        //         Some(MaybeGraph::Started) => "Started",
        //         Some(MaybeGraph::Inlined) => "Inlined",
        //         Some(MaybeGraph::Finished(_)) => "Finished",
        //         None => "None",
        //     }
        // );
        if let Some(MaybeGraph::Inlined | MaybeGraph::Finished(_)) = graphs.get(step) {
            // we've already visited this step.
            // crate::log(format!("visited step {} but it is already visited", step.id()).as_str());
            return Ok(());
        }
        // crate::log(format!("visited step {}, not yet visited", step.id()).as_str());
        // crate::log!(
        //     "currently visited/started steps: {:?}",
        //     graphs.keys().map(|step| step.id()).collect::<Box<[_]>>()
        // );
        // crate::log!("hash of step: {:?}", graphs.);
        let maybe_proc_context = step.context().proc_context.as_ref();

        let mut should_propagate_ssa = step.used_non_inline() && maybe_proc_context.is_none();

        let mut opcode_replacements: Vec<(usize, IrOpcode)> = vec![];
        let mut additional_opcodes: Vec<(usize, Vec<IrOpcode>)> = vec![];
        'opcode_loop: for (i, opcode) in step.opcodes().try_borrow()?.iter().enumerate() {
            // crate::log!("opcode: {opcode:?}");
            // let's just assume that input types match up.
            #[expect(clippy::wildcard_enum_match_arm, reason = "too many variants to match")]
            match opcode {
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var,
                    local_write: locality,
                }) => {
                    // crate::log!("found a variable write operation, type stack: {type_stack:?}");
                    let already_local = *locality.try_borrow()?;
                    if already_local {
                        type_stack.clear();
                        continue 'opcode_loop;
                    }
                    let new_variable = RcVar::new(IrType::none(), VarVal::Bool(false));
                    {
                        variable_maps
                            .ssa
                            .insert(var.try_borrow()?.clone(), new_variable.clone());
                    }
                    *var.try_borrow_mut()? = new_variable;
                    *locality.try_borrow_mut()? = true;
                    if type_stack.is_empty() {
                        let mut graph = self.graph().borrow_mut();
                        let Some(Some((vars, _))) =
                            graph.node_weight_mut(*self.exit_node().borrow())
                        else {
                            hq_bug!("")
                        };
                        vars.push(VarOrList::Var(var.try_borrow()?.clone()));
                    } else if !type_stack.is_empty() {
                        let new_node = self.add_node(Some((
                            vec![VarOrList::Var(var.try_borrow()?.clone())],
                            mem::take(type_stack),
                        )));
                        let last_node = *self.exit_node().borrow();
                        self.add_edge(last_node, new_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = new_node;
                    }
                    //     crate::log!(
                    //         "{:?}exit node: {:?}",
                    //         Dot::with_config(
                    //             &*self.graph().borrow(),
                    //             &[
                    // //DotConfig::NodeIndexLabel
                    // ]
                    //         ),
                    //         *self.exit_node().borrow()
                    //     );
                    // crate::log!("type stack: {type_stack:?}");
                }
                IrOpcode::data_teevariable(DataTeevariableFields {
                    var,
                    local_read_write: locality,
                }) => {
                    // crate::log("found a variable tee operation");
                    let already_local = *locality.try_borrow()?;
                    if already_local {
                        *type_stack = vec![StackElement::Var(var.try_borrow()?.clone())];
                        continue 'opcode_loop;
                    }
                    let new_variable = RcVar::new(IrType::none(), VarVal::Bool(false));
                    {
                        variable_maps
                            .ssa
                            .insert(var.try_borrow()?.clone(), new_variable.clone());
                    }
                    *var.try_borrow_mut()? = new_variable;
                    *locality.try_borrow_mut()? = true;
                    // TODO: do we need to consider the case where the stack is empty,
                    // as with setvariableto?
                    let new_node = self.add_node(Some((
                        vec![VarOrList::Var(var.try_borrow()?.clone())],
                        mem::take(type_stack),
                    )));
                    let last_node = *self.exit_node().borrow();
                    self.add_edge(last_node, new_node, EdgeType::Forward);
                    *self.exit_node().borrow_mut() = new_node;
                    type_stack.push(StackElement::Var(var.try_borrow()?.clone()));
                }
                IrOpcode::data_variable(DataVariableFields { var, local_read }) => {
                    // crate::log("found a variable read operation");
                    if *local_read.try_borrow()? {
                        type_stack.push(StackElement::Var(var.try_borrow()?.clone()));
                        // crate::log("variable is already local; skipping");
                        continue;
                    }
                    let gvar = &var.try_borrow()?.clone();
                    let mut var_mut = var.try_borrow_mut()?;
                    type_stack.push(StackElement::Var(
                        variable_maps.use_current_ssa_for_global::<_, _, _, HQResult<_>>(
                            gvar,
                            |current_ssa_var| {
                                *var_mut = current_ssa_var.clone();
                                *local_read.try_borrow_mut()? = true;
                                Ok(current_ssa_var.clone())
                            },
                            |var_index, arg_var| {
                                opcode_replacements.push((
                                    i,
                                    IrOpcode::procedures_argument(ProceduresArgumentFields(
                                        var_index,
                                        arg_var.clone(),
                                    )),
                                ));
                                Ok(arg_var.clone())
                            },
                            || Ok(gvar.clone()),
                        )?,
                    ));
                    // crate::log!("stack: {type_stack:?}");
                }
                IrOpcode::data_itemoflist(DataItemoflistFields { list }) => {
                    type_stack.push(StackElement::List(list.clone()));
                }
                IrOpcode::data_addtolist(DataAddtolistFields { list })
                | IrOpcode::data_replaceitemoflist(DataReplaceitemoflistFields { list }) => {
                    let new_node = self.add_node(Some((
                        vec![VarOrList::List(list.clone())],
                        mem::take(type_stack),
                    )));
                    let last_node = *self.exit_node().borrow();
                    self.add_edge(last_node, new_node, EdgeType::Forward);
                    *self.exit_node().borrow_mut() = new_node;
                }
                IrOpcode::control_if_else(ControlIfElseFields {
                    branch_if,
                    branch_else,
                }) => {
                    // crate::log("found control_if_else");
                    type_stack.clear(); // the top item on the stack should be consumed by control_if_else
                    let last_node = *self.exit_node().borrow();

                    let mut branch_if_variable_maps = variable_maps.clone();
                    graphs.insert(Rc::clone(branch_if), MaybeGraph::Started);
                    self.visit_step(
                        branch_if,
                        &mut branch_if_variable_maps,
                        graphs,
                        type_stack,
                        next_steps,
                    )?;
                    graphs.insert(Rc::clone(branch_if), MaybeGraph::Inlined);
                    let mut if_branch_exit = *self.exit_node().borrow();
                    *self.exit_node().borrow_mut() = last_node;

                    let mut branch_else_variable_maps = variable_maps.clone();
                    graphs.insert(Rc::clone(branch_else), MaybeGraph::Started);
                    self.visit_step(
                        branch_else,
                        &mut branch_else_variable_maps,
                        graphs,
                        type_stack,
                        next_steps,
                    )?;
                    graphs.insert(Rc::clone(branch_else), MaybeGraph::Inlined);
                    let mut else_branch_exit = *self.exit_node().borrow();
                    *self.exit_node().borrow_mut() = self.add_node(None);

                    self.ssa_phi(
                        vec![
                            (branch_if, branch_if_variable_maps.ssa, &mut if_branch_exit),
                            (
                                branch_else,
                                branch_else_variable_maps.ssa,
                                &mut else_branch_exit,
                            ),
                        ],
                        variable_maps,
                        &BTreeMap::new(),
                    )?;

                    let exit_node = *self.exit_node().borrow();
                    self.add_edge(if_branch_exit, exit_node, EdgeType::Forward);
                    self.add_edge(else_branch_exit, exit_node, EdgeType::Forward);
                    type_stack.clear();
                }
                IrOpcode::control_loop(ControlLoopFields {
                    first_condition,
                    condition,
                    body,
                    ..
                }) => {
                    let new_var_map: BTreeMap<_, _> = step
                        .globally_scoped_variables()?
                        .map(|global_var| {
                            (global_var, RcVar::new(IrType::none(), VarVal::Bool(false)))
                        })
                        .collect();
                    additional_opcodes.push((
                        i,
                        new_var_map
                            .iter()
                            .flat_map(|(global_var, new_var)| {
                                let (var_access, accessed_var) = variable_maps
                                    .use_current_ssa_for_global(
                                        global_var,
                                        |ssa_var| {
                                            (
                                                IrOpcode::data_variable(DataVariableFields {
                                                    var: RefCell::new(ssa_var.clone()),
                                                    local_read: RefCell::new(true),
                                                }),
                                                ssa_var.clone(),
                                            )
                                        },
                                        |arg_index, arg_var| {
                                            (
                                                IrOpcode::procedures_argument(
                                                    ProceduresArgumentFields(
                                                        arg_index,
                                                        arg_var.clone(),
                                                    ),
                                                ),
                                                arg_var.clone(),
                                            )
                                        },
                                        || {
                                            (
                                                IrOpcode::data_variable(DataVariableFields {
                                                    var: RefCell::new(global_var.clone()),
                                                    local_read: RefCell::new(false),
                                                }),
                                                global_var.clone(),
                                            )
                                        },
                                    );
                                let new_node = self.add_node(Some((
                                    vec![VarOrList::Var(new_var.clone())],
                                    vec![StackElement::Var(accessed_var)],
                                )));
                                let exit_node = *self.exit_node().borrow();
                                self.add_edge(exit_node, new_node, EdgeType::Forward);
                                *self.exit_node().borrow_mut() = new_node;
                                variable_maps
                                    .ssa
                                    .insert(global_var.clone(), new_var.clone());
                                [
                                    var_access,
                                    IrOpcode::data_setvariableto(DataSetvariabletoFields {
                                        var: RefCell::new(new_var.clone()),
                                        local_write: RefCell::new(true),
                                    }),
                                ]
                            })
                            .collect(),
                    ));
                    if let Some(first_condition_step) = first_condition {
                        graphs.insert(Rc::clone(first_condition_step), MaybeGraph::Started);
                        self.visit_step(
                            first_condition_step,
                            // we don't need to keep track of this map as no variables should be set in the first condition
                            &mut variable_maps.clone(),
                            graphs,
                            type_stack,
                            next_steps,
                        )?;
                        graphs.insert(Rc::clone(first_condition_step), MaybeGraph::Inlined);
                        type_stack.clear(); // we consume the top item on the type stack as an i32.eqz
                        let first_cond_exit = *self.exit_node().borrow();

                        let header_node = self.add_node(None);
                        self.add_edge(first_cond_exit, header_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = header_node;

                        let mut body_variable_maps = variable_maps.clone();
                        graphs.insert(Rc::clone(body), MaybeGraph::Started);
                        self.visit_step(
                            body,
                            &mut body_variable_maps,
                            graphs,
                            type_stack,
                            next_steps,
                        )?;
                        graphs.insert(Rc::clone(body), MaybeGraph::Inlined);
                        graphs.insert(Rc::clone(condition), MaybeGraph::Started);
                        self.visit_step(
                            condition,
                            &mut body_variable_maps,
                            graphs,
                            type_stack,
                            next_steps,
                        )?;
                        graphs.insert(Rc::clone(condition), MaybeGraph::Inlined);
                        type_stack.clear();
                        let mut cond_exit = *self.exit_node().borrow();

                        self.ssa_phi(
                            vec![(condition, body_variable_maps.ssa, &mut cond_exit)],
                            variable_maps,
                            &new_var_map,
                        )?;

                        self.add_edge(cond_exit, header_node, EdgeType::BackLink);
                        *self.exit_node().borrow_mut() = cond_exit;
                    } else {
                        let header_node = self.add_node(None);
                        let last_node = *self.exit_node().borrow();
                        self.add_edge(last_node, header_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = header_node;

                        let mut body_variable_maps = variable_maps.clone();
                        graphs.insert(Rc::clone(condition), MaybeGraph::Started);
                        self.visit_step(
                            condition,
                            &mut body_variable_maps,
                            graphs,
                            type_stack,
                            next_steps,
                        )?;
                        graphs.insert(Rc::clone(condition), MaybeGraph::Inlined);
                        type_stack.clear();
                        graphs.insert(Rc::clone(body), MaybeGraph::Started);
                        self.visit_step(
                            body,
                            &mut body_variable_maps,
                            graphs,
                            type_stack,
                            next_steps,
                        )?;
                        graphs.insert(Rc::clone(body), MaybeGraph::Inlined);
                        let mut body_exit = *self.exit_node().borrow();

                        self.ssa_phi(
                            vec![(body, body_variable_maps.ssa, &mut body_exit)],
                            variable_maps,
                            &new_var_map,
                        )?;

                        self.add_edge(body_exit, header_node, EdgeType::BackLink);
                        *self.exit_node().borrow_mut() = body_exit;
                    }
                    let loop_exit = *self.exit_node().borrow();
                    let new_node = self.add_node(None);
                    self.add_edge(loop_exit, new_node, EdgeType::Forward);
                    *self.exit_node().borrow_mut() = new_node;
                }
                IrOpcode::procedures_argument(ProceduresArgumentFields(_, var)) => {
                    // crate::log("found proc argument.");
                    type_stack.push(StackElement::Var(var.clone()));
                }
                IrOpcode::procedures_call_warp(ProceduresCallWarpFields { proc }) => {
                    // crate::log!("found proc call. type stack: {type_stack:?}");
                    let entry_node = self.add_node(Some((
                        proc.context()
                            .arg_vars()
                            .try_borrow()?
                            .iter()
                            .rev()
                            .cloned()
                            .map(VarOrList::Var)
                            .collect(),
                        mem::take(type_stack),
                    )));
                    let last_node = *self.exit_node().borrow();
                    self.add_edge(last_node, entry_node, EdgeType::Forward);
                    *self.exit_node().borrow_mut() = entry_node;
                    type_stack.extend(
                        proc.context()
                            .return_vars()
                            .try_borrow()?
                            .iter()
                            .map(|var| StackElement::Var(var.clone())),
                    );
                    // crate::log!("type stack after proc call: {type_stack:?}");
                }
                IrOpcode::hq_yield(HqYieldFields { mode }) => match mode {
                    YieldMode::Inline(step) => {
                        // crate::log("found inline step to visit");
                        graphs.insert(Rc::clone(step), MaybeGraph::Started);
                        self.visit_step(step, variable_maps, graphs, type_stack, next_steps)?;
                        graphs.insert(Rc::clone(step), MaybeGraph::Inlined);
                    }
                    YieldMode::None => {
                        // crate::log("found a yield::none, breaking");
                        should_propagate_ssa = true;
                        break 'opcode_loop;
                    }
                    YieldMode::Schedule(step) => {
                        // crate::log("found a scheduled step, scheduling and breaking");
                        if let Some(rcstep) = step.upgrade() {
                            next_steps.push(rcstep);
                        } else {
                            crate::warn(
                                "couldn't upgrade Weak<Step> in Schedule in variables optimisation pass;\
                    what's going on here?",
                            );
                        }
                        should_propagate_ssa = true;
                        break 'opcode_loop;
                    }
                },
                _ => {
                    let inputs_len = opcode.acceptable_inputs()?.len();
                    let output = opcode
                        .output_type(core::iter::repeat_n(IrType::Any, inputs_len).collect())?;
                    match output {
                        ReturnType::Singleton(output_ty) => {
                            // crate::log("other opcode; has output type");
                            if inputs_len == 0 {
                                type_stack.push(StackElement::Type(output_ty));
                                // crate::log("no inputs so pushing type");
                                // crate::log!("stack: {type_stack:?}");
                            } else {
                                type_stack.push(StackElement::Opcode(opcode.clone()));
                                // crate::log("inputs so pushing op");
                                // crate::log!("stack: {type_stack:?}");
                            }
                        }
                        ReturnType::MultiValue(outputs) => {
                            if inputs_len == 0 {
                                type_stack.extend(outputs.iter().copied().map(StackElement::Type));
                            } else {
                                type_stack.push(StackElement::Opcode(opcode.clone()));
                            }
                        }
                        ReturnType::None => type_stack.clear(),
                    }
                }
            }
        }

        if !type_stack.is_empty()
            && step.used_non_inline()
            && let Some(proc_context) = maybe_proc_context
        {
            // crate::log!("found spare items on type stack at end of step visit: {type_stack:?}");
            // crate::log!(
            //     "return vars: {}",
            //     proc_context.return_vars().try_borrow()?.len()
            // );
            let new_node = self.add_node(Some((
                proc_context
                    .return_vars()
                    .try_borrow()?
                    .iter()
                    .rev()
                    .cloned()
                    .map(VarOrList::Var)
                    .collect(),
                mem::take(type_stack),
            )));
            let last_node = *self.exit_node().borrow();
            self.add_edge(last_node, new_node, EdgeType::Forward);
            *self.exit_node().borrow_mut() = new_node;
        }

        {
            let mut opcodes = step.opcodes_mut()?;

            for (index, opcode_replacement) in opcode_replacements {
                *opcodes
                    .get_mut(index)
                    .ok_or_else(|| make_hq_bug!("opcode index out of bounds"))? =
                    opcode_replacement;
            }

            for (index, additional_opcodes) in additional_opcodes {
                opcodes.splice(index..index, additional_opcodes);
            }
        }

        if should_propagate_ssa {
            let post_yield = if let Some(last_op) = step.opcodes().try_borrow()?.last()
                && matches!(last_op, IrOpcode::hq_yield(_))
            {
                true
            } else {
                false
            };
            let mut opcodes = step.opcodes_mut()?;
            let yield_op = if post_yield {
                #[expect(clippy::unwrap_used, reason = "guaranteed last element")]
                Some(opcodes.pop().unwrap())
            } else {
                None
            };
            for (global_var, ssa_var) in &variable_maps.ssa {
                let this_type_stack = vec![StackElement::Var(ssa_var.clone())];
                let new_node = self.add_node(Some((
                    vec![VarOrList::Var(global_var.clone())],
                    this_type_stack,
                )));
                let last_node = *self.exit_node().borrow();
                self.add_edge(last_node, new_node, EdgeType::Forward);
                *self.exit_node().borrow_mut() = new_node;
                // crate::log("adding outer variable writes");
                opcodes.extend([
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(ssa_var.clone()),
                        local_read: RefCell::new(true),
                    }),
                    IrOpcode::data_setvariableto(DataSetvariabletoFields {
                        var: RefCell::new(global_var.clone()),
                        local_write: RefCell::new(false),
                    }),
                ]);
            }
            if post_yield {
                #[expect(clippy::unwrap_used, reason = "guaranteed to be Some")]
                opcodes.push(yield_op.unwrap());
            }
        }
        Ok(())
    }

    /// Insert an SSA "phi function"; that is, propagate any new SSAs to the outer scope,
    /// at a so-called dominance frontier (where different paths meet)
    fn ssa_phi(
        &self,
        mut ssa_blocks: Vec<(&Rc<Step>, BTreeMap<RcVar, RcVar>, &mut NodeIndex)>,
        variable_maps: &mut VariableMaps,
        ssa_write_map: &BTreeMap<RcVar, RcVar>,
    ) -> HQResult<()> {
        let mut lower_ssas: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();

        for (block, block_ssa, _) in &ssa_blocks {
            for (global, block_local) in block_ssa {
                lower_ssas
                    .entry(global.clone())
                    .and_modify(|ssa_block_map| {
                        ssa_block_map.insert(Rc::clone(block), block_local.clone());
                    })
                    .or_insert_with(|| BTreeMap::from([(Rc::clone(block), block_local.clone())]));
            }
        }

        let mut block_exits: BTreeMap<_, _> = ssa_blocks
            .iter_mut()
            .map(|(block, _block_ssas, block_exit)| (Rc::clone(block), block_exit))
            .collect();

        let mut block_var_writes = BTreeMap::new();

        for (global, block_ssas) in lower_ssas {
            let new_var = ssa_write_map
                .get(&global)
                .cloned()
                .unwrap_or_else(|| RcVar::new(IrType::none(), VarVal::Bool(false)));
            for block in block_exits.keys() {
                if !block_ssas.contains_key(block) {
                    block_var_writes
                        .entry(Rc::clone(block))
                        .or_insert_with(Vec::new)
                        .push((
                            variable_maps
                                .ssa
                                .get(&global)
                                .cloned()
                                .map_or_else(|| (global.clone(), false), |var| (var, true)),
                            new_var.clone(),
                        ));
                }
            }
            for (block, block_ssa) in &block_ssas {
                block_var_writes
                    .entry(Rc::clone(block))
                    .or_insert_with(Vec::new)
                    .push(((block_ssa.clone(), true), new_var.clone()));
            }
            variable_maps.ssa.insert(global.clone(), new_var.clone());
        }

        for (block, var_writes) in &block_var_writes {
            let Some(block_exit) = block_exits.get_mut(block) else {
                hq_bug!("couldn't find SSA block exit")
            };
            let mut opcodes = block.opcodes_mut()?;
            opcodes.reserve_exact(var_writes.len() * 2);
            for ((var_read, read_local), var_write) in var_writes {
                let new_node = self.add_node(Some((
                    vec![VarOrList::Var(var_write.clone())],
                    vec![StackElement::Var(var_read.clone())],
                )));
                self.add_edge(***block_exit, new_node, EdgeType::Forward);
                ***block_exit = new_node;

                opcodes.append(&mut vec![
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(var_read.clone()),
                        local_read: RefCell::new(*read_local),
                    }),
                    IrOpcode::data_setvariableto(DataSetvariabletoFields {
                        var: RefCell::new(var_write.clone()),
                        local_write: RefCell::new(true),
                    }),
                ]);
            }
        }

        Ok(())
    }
}

fn visit_step_recursively(
    step: Rc<Step>,
    graphs: &mut BTreeMap<Rc<Step>, MaybeGraph>,
    proc_args: &BTreeMap<RcVar, (usize, RcVar)>,
) -> HQResult<()> {
    let next_steps = &mut vec![step];
    while let Some(next_step) = next_steps.pop() {
        let entry = graphs.entry(Rc::clone(&next_step));
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
                &next_step,
                &mut VariableMaps::new_with_proc_args(proc_args),
                graphs,
                &mut vec![],
                next_steps,
            )?;
            graphs.insert(Rc::clone(&next_step), MaybeGraph::Finished(graph));
            // crate::log!("finished graph for step {}", next_step.id());
        }
    }
    Ok(())
}

fn visit_procedure(proc: &Rc<Proc>, graphs: &mut BTreeMap<Rc<Step>, MaybeGraph>) -> HQResult<()> {
    // crate::log("visiting procedure");
    if let PartialStep::Finished(step) = proc.warped_first_step()?.clone() {
        // crate::log!("visiting procedure's step: {}", step.id());
        let globally_scoped_variables: Box<[_]> = step.globally_scoped_variables()?.collect();
        let arg_vars_drop =
            proc.context().arg_vars().try_borrow()?.len() - globally_scoped_variables.len();
        visit_step_recursively(
            Rc::clone(&step),
            graphs,
            &globally_scoped_variables
                .into_iter()
                .zip(
                    proc.context()
                        .arg_vars()
                        .try_borrow()?
                        .iter()
                        .cloned()
                        .enumerate()
                        .dropping(arg_vars_drop),
                )
                .collect(),
        )?;
    }
    Ok(())
}

/// Splits variables at every write site, and ensures that outer/global variables are then written
/// to at the end of a step. This also constructs variable type graphs for each step which can then
/// be analyzed to determine the best types for variables.
fn split_variables_and_make_graphs(
    project: &Rc<IrProject>,
) -> HQResult<BTreeMap<Rc<Step>, MaybeGraph>> {
    // crate::log("splitting variables and making graphs");
    #[expect(
        clippy::mutable_key_type,
        reason = "implementation of Ord for Step relies on `id` field only, which is immutable"
    )]
    let mut graphs = BTreeMap::new();
    for (_, target) in project.targets().borrow().iter() {
        for (_, proc) in target.procedures()?.iter() {
            visit_procedure(proc, &mut graphs)?;
        }
    }
    for thread in project.threads().try_borrow()?.iter() {
        let step = Rc::clone(thread.first_step());
        // crate::log!("visiting (recursively) step from thread: {}", step.id());
        visit_step_recursively(step, &mut graphs, &BTreeMap::new())?;
    }
    // crate::log("finished splitting variables and making graphs");
    Ok(graphs)
}

fn evaluate_type_stack(type_stack: &Vec<StackElement>) -> HQResult<Vec<IrType>> {
    let mut stack = vec![];
    // crate::log!("type stack: {type_stack:?}");
    for el in type_stack {
        match el {
            StackElement::Opcode(op) => {
                let inputs_len = op.acceptable_inputs()?.len();
                // crate::log!("opcode: {op}");
                // crate::log!("stack length: {}, inputs len: {}", stack.len(), inputs_len);
                // crate::log!("stack: {:?}", stack);
                let inputs: Rc<[_]> = stack
                    .splice((stack.len() - inputs_len).., [])
                    .map(|ty: IrType| if ty.is_none() { IrType::Any } else { ty })
                    .collect();
                let output = op.output_type(inputs)?;
                match output {
                    ReturnType::Singleton(ty) => stack.push(ty),
                    ReturnType::MultiValue(tys) => stack.extend(tys.iter().copied()),
                    ReturnType::None => (),
                }
            }
            StackElement::Type(ty) => stack.push(*ty),
            StackElement::Var(var) => stack.push(*var.possible_types()),
            StackElement::List(list) => stack.push(*list.possible_types()),
        }
    }
    Ok(stack)
}

fn visit_node(
    graph: &VarGraph,
    node: NodeIndex,
    changed_vars: &mut BTreeSet<VarOrList>,
    stop_at: NodeIndex,
) -> HQResult<()> {
    // crate::log!("node index: {node:?}");
    if let Some(Some((vars, type_stack))) = graph.graph().try_borrow()?.node_weight(node) {
        // crate::log!("vars: {vars:?}");
        let reduced_stack = evaluate_type_stack(type_stack)?;
        // crate::log!("stack: {type_stack:?}\nreduced stack: {reduced_stack:?}");
        hq_assert_eq!(
            vars.len(),
            reduced_stack.len(),
            "variables stack should have the same length as the corresponding type stack"
        );
        for (var, new_type) in vars.iter().rev().zip(reduced_stack) {
            if !var.possible_types().contains(new_type) {
                changed_vars.insert(var.clone());
            }
            var.add_type(new_type);
        }
    }
    if node == stop_at {
        return Ok(());
    }
    let inner_graph = graph.graph().try_borrow()?;
    let edges = inner_graph.edges_directed(node, EdgeOut);
    let (backlinks, forlinks): (Vec<_>, _) =
        edges.partition(|edge| *edge.weight() == EdgeType::BackLink);
    hq_assert!(
        backlinks.len() <= 1,
        "Each node should have at most 1 backlink"
    );
    if let Some(backlink) = backlinks.first() {
        loop {
            let mut loop_changed_vars = BTreeSet::new();
            visit_node(graph, backlink.target(), &mut loop_changed_vars, node)?;
            if loop_changed_vars.is_empty() {
                break;
            }
        }
    }
    for forlink in forlinks {
        visit_node(graph, forlink.target(), changed_vars, stop_at)?;
    }
    Ok(())
}

fn iterate_graphs<'a, I>(graphs: &'a I) -> HQResult<()>
where
    I: Iterator<Item = &'a VarGraph> + Clone,
{
    loop {
        let mut changed_vars: BTreeSet<VarOrList> = BTreeSet::new();
        for graph in graphs.clone() {
            // crate::log!(
            //     "{:?}exit node: {:?}",
            //     Dot::with_config(
            //         &*graph.graph().borrow(),
            //         &[
            //         //DotConfig::NodeIndexLabel
            //         ]
            //     ),
            //     *graph.exit_node().borrow()
            // );
            hq_assert!(
                {
                    let inner_graph = graph.graph().try_borrow()?;
                    let mut externals = inner_graph.externals(EdgeIn);
                    externals.next().is_some_and(|e| e == NodeIndex::new(0))
                        && externals.next().is_none()
                },
                "Variable graph should only have one source node, at index 0"
            );
            let first_node = NodeIndex::new(0);
            visit_node(
                graph,
                first_node,
                &mut changed_vars,
                *graph.exit_node().try_borrow()?,
            )?;
        }
        // this must eventually converge, because the sequence of types for each variable at each
        // iteration is increasing, but the set of types is finite
        if changed_vars.is_empty() {
            break;
        }
    }
    Ok(())
}

/// A token type that cannot be instantiated from anywhere else (since the field is private)
/// - used as proof that we've carried out these optimisations.
pub struct SSAToken(PhantomData<()>);

pub fn optimise_variables(project: &Rc<IrProject>) -> HQResult<SSAToken> {
    // crate::log("carrying out variable optimisation");
    let maybe_graphs = split_variables_and_make_graphs(project)?;
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
    // crate::log!("graphs num: {}", graphs.len());
    iterate_graphs(&graphs.values().copied())?;
    for step in maybe_graphs.keys() {
        // crate::log!("inserting casts for step {}", step.id());
        insert_casts(&mut *step.opcodes_mut()?, false)?;
    }
    Ok(SSAToken(PhantomData))
}
