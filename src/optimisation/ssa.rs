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

use alloc::collections::btree_map::Entry;
use core::convert::identity;
use core::hash::{Hash, Hasher};
use core::marker::PhantomData;
use core::ops::Deref;

use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::stable_graph::StableDiGraph;
use petgraph::visit::EdgeRef;
use petgraph::{Incoming as EdgeIn, Outgoing as EdgeOut};

use crate::instructions::{
    ControlIfElseFields, ControlLoopFields, DataAddtolistFields, DataInsertatlistFields,
    DataItemoflistFields, DataReplaceitemoflistFields, DataSetvariabletoFields,
    DataTeevariableFields, DataVariableFields, HqBoxFields, HqYieldFields, IrOpcode,
    ProceduresArgumentFields, ProceduresCallNonwarpFields, ProceduresCallWarpFields, YieldMode,
};
use crate::ir::{
    InlinedStep, IrProject, PartialStep, Proc, RcList, RcVar, ReturnType, Step, StepIndex,
    Type as IrType, insert_casts,
};
use crate::prelude::*;
use crate::wasm::flags::{Switch, VarTypeConvergence};

#[derive(Clone, Debug)]
enum StackOperation {
    Drop,
    Push(VarTarget),
    Pop(VarTarget),
    /// this shouldn't be anything that branches or mutates variables in any way,
    /// i.e. not loop, if/else, yield, set variable, etc
    Opcode(IrOpcode),
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
enum VarTarget {
    Var(RcVar),
    List(RcList),
}

impl VarTarget {
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

type NodeWeight = Option<StackOperation>;

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
        let first_node = graph.add_node(None);
        let exit_node = graph.add_node(None);
        graph.add_edge(first_node, exit_node, EdgeType::Forward);
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

    fn add_node_at_end(&self, weight: NodeWeight) {
        let new_node = self.add_node(weight);
        let end_node = *self.exit_node().borrow();
        self.add_edge(end_node, new_node, EdgeType::Forward);
        *self.exit_node().borrow_mut() = new_node;
    }

    fn add_edge(&self, a: NodeIndex, b: NodeIndex, weight: EdgeType) -> EdgeIndex {
        self.graph().borrow_mut().add_edge(a, b, weight)
    }

    /// Visits a step to split its variables and construct a variable graph for it.
    fn visit_step<S>(
        &mut self,
        step: S,
        variable_maps: &mut VariableMaps<'_>,
        graphs: &mut BTreeMap<Box<str>, MaybeGraph>,
        next_steps: &mut Vec<StepIndex>,
        do_ssa: bool,
    ) -> HQResult<()>
    where
        S: Deref<Target = RefCell<Step>>,
    {
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
        if let Some(MaybeGraph::Inlined | MaybeGraph::Finished(_)) =
            graphs.get(step.try_borrow()?.id())
        {
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

        let maybe_proc_context = {
            let step_tmp = step.try_borrow()?;
            step_tmp.context().proc_context.clone()
        };

        let mut should_propagate_ssa =
            step.try_borrow()?.used_non_inline() && maybe_proc_context.is_none();
        let mut step_ended_on_stop = false;

        let mut opcode_replacements: Vec<(usize, IrOpcode)> = vec![];
        let mut additional_opcodes: Vec<(usize, Vec<IrOpcode>)> = vec![];
        'opcode_loop: for (i, opcode) in step.try_borrow()?.opcodes().iter().enumerate() {
            // crate::log!("opcode: {opcode:?}");
            // let's just assume that input types match up.
            #[expect(clippy::wildcard_enum_match_arm, reason = "too many variants to match")]
            match opcode {
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var,
                    local_write: locality,
                    first_write,
                }) => {
                    // crate::log!("found a variable write operation, type stack: {type_stack:?}");
                    let already_local = *locality.try_borrow()?;
                    if already_local {
                        if var.try_borrow()?.possible_types().is_none() {
                            let pop_node = self.add_node(Some(StackOperation::Pop(
                                VarTarget::Var(var.try_borrow()?.clone()),
                            )));
                            let last_node = *self.exit_node().borrow();
                            self.add_edge(last_node, pop_node, EdgeType::Forward);
                            *self.exit_node().borrow_mut() = pop_node;
                        } else {
                            let drop_node = self.add_node(Some(StackOperation::Drop));
                            let last_node = *self.exit_node().borrow();
                            self.add_edge(last_node, drop_node, EdgeType::Forward);
                            *self.exit_node().borrow_mut() = drop_node;
                        }
                        continue 'opcode_loop;
                    }
                    if !do_ssa {
                        let pop_node = self.add_node(Some(StackOperation::Pop(VarTarget::Var(
                            var.try_borrow()?.clone(),
                        ))));
                        let last_node = *self.exit_node().borrow();
                        self.add_edge(last_node, pop_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = pop_node;
                        continue 'opcode_loop;
                    }
                    let new_variable = RcVar::new_empty();
                    {
                        variable_maps
                            .ssa
                            .insert(var.try_borrow()?.clone(), new_variable.clone());
                    }
                    *var.try_borrow_mut()? = new_variable;
                    *first_write.try_borrow_mut()? = true;
                    *locality.try_borrow_mut()? = true;
                    let pop_node = self.add_node(Some(StackOperation::Pop(VarTarget::Var(
                        var.try_borrow()?.clone(),
                    ))));
                    let last_node = *self.exit_node().borrow();
                    self.add_edge(last_node, pop_node, EdgeType::Forward);
                    *self.exit_node().borrow_mut() = pop_node;
                }
                IrOpcode::data_teevariable(DataTeevariableFields {
                    var,
                    local_read_write: locality,
                }) => {
                    // crate::log("found a variable tee operation");
                    let already_local = *locality.try_borrow()?;
                    if !do_ssa || already_local {
                        self.add_node_at_end(Some(StackOperation::Push(VarTarget::Var(
                            var.try_borrow()?.clone(),
                        ))));
                        continue 'opcode_loop;
                    }
                    let new_variable = RcVar::new_empty();
                    {
                        variable_maps
                            .ssa
                            .insert(var.try_borrow()?.clone(), new_variable.clone());
                    }
                    *var.try_borrow_mut()? = new_variable;
                    *locality.try_borrow_mut()? = true;
                    self.add_node_at_end(Some(StackOperation::Pop(VarTarget::Var(
                        var.try_borrow()?.clone(),
                    ))));
                    self.add_node_at_end(Some(StackOperation::Push(VarTarget::Var(
                        var.try_borrow()?.clone(),
                    ))));
                }
                IrOpcode::data_variable(DataVariableFields { var, local_read }) => {
                    // crate::log("found a variable read operation");
                    if *local_read.try_borrow()? {
                        self.add_node_at_end(Some(StackOperation::Push(VarTarget::Var(
                            var.try_borrow()?.clone(),
                        ))));
                        // crate::log("variable is already local; skipping");
                        continue;
                    }
                    let gvar = &var.try_borrow()?.clone();
                    let mut var_mut = var.try_borrow_mut()?;
                    self.add_node_at_end(Some(StackOperation::Push(VarTarget::Var(
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
                                    IrOpcode::procedures_argument(ProceduresArgumentFields {
                                        index: var_index,
                                        arg_var: arg_var.clone(),
                                        in_warped: true,
                                        arg_vars: Rc::clone(
                                            &maybe_proc_context
                                                .as_ref()
                                                .ok_or_else(|| {
                                                    make_hq_bug!(
                                                        "tried to access proc argument when proc \
                                                         context was None"
                                                    )
                                                })?
                                                .arg_vars,
                                        ),
                                    }),
                                ));
                                Ok(arg_var.clone())
                            },
                            || Ok(gvar.clone()),
                        )?,
                    ))));
                }
                IrOpcode::data_itemoflist(DataItemoflistFields { list }) => {
                    self.add_node_at_end(Some(StackOperation::Drop));
                    self.add_node_at_end(Some(StackOperation::Push(VarTarget::List(list.clone()))));
                }
                IrOpcode::data_addtolist(DataAddtolistFields { list }) => {
                    self.add_node_at_end(Some(StackOperation::Pop(VarTarget::List(list.clone()))));
                }
                IrOpcode::data_replaceitemoflist(DataReplaceitemoflistFields { list })
                | IrOpcode::data_insertatlist(DataInsertatlistFields { list }) => {
                    self.add_node_at_end(Some(StackOperation::Pop(VarTarget::List(list.clone()))));
                    self.add_node_at_end(Some(StackOperation::Drop));
                }
                IrOpcode::control_if_else(ControlIfElseFields {
                    branch_if,
                    branch_else,
                }) => {
                    // crate::log("found control_if_else");
                    // the top item on the stack should be consumed by control_if_else
                    self.add_node_at_end(Some(StackOperation::Drop));
                    let last_node = *self.exit_node().borrow();

                    let branch_if_mut = Rc::new(Rc::unwrap_or_clone(Rc::clone(branch_if)));
                    let branch_else_mut = Rc::new(Rc::unwrap_or_clone(Rc::clone(branch_else)));

                    let mut branch_if_variable_maps = variable_maps.clone();
                    graphs.insert(branch_if_mut.try_borrow()?.id().into(), MaybeGraph::Started);
                    self.visit_step(
                        Rc::clone(&branch_if_mut),
                        &mut branch_if_variable_maps,
                        graphs,
                        next_steps,
                        do_ssa,
                    )?;
                    graphs.insert(branch_if_mut.try_borrow()?.id().into(), MaybeGraph::Inlined);
                    let mut if_branch_exit = *self.exit_node().borrow();
                    *self.exit_node().borrow_mut() = last_node;

                    let mut branch_else_variable_maps = variable_maps.clone();
                    graphs.insert(
                        branch_else_mut.try_borrow()?.id().into(),
                        MaybeGraph::Started,
                    );
                    self.visit_step(
                        Rc::clone(&branch_else_mut),
                        &mut branch_else_variable_maps,
                        graphs,
                        next_steps,
                        do_ssa,
                    )?;
                    graphs.insert(
                        branch_else_mut.try_borrow()?.id().into(),
                        MaybeGraph::Inlined,
                    );
                    let mut else_branch_exit = *self.exit_node().borrow();
                    *self.exit_node().borrow_mut() = self.add_node(None);

                    if do_ssa {
                        self.ssa_phi(
                            vec![
                                (
                                    &branch_if_mut,
                                    branch_if_variable_maps.ssa,
                                    &mut if_branch_exit,
                                ),
                                (
                                    &branch_else_mut,
                                    branch_else_variable_maps.ssa,
                                    &mut else_branch_exit,
                                ),
                            ],
                            variable_maps,
                            &BTreeMap::new(),
                        )?;
                    }

                    opcode_replacements.push((
                        i,
                        IrOpcode::control_if_else(ControlIfElseFields {
                            branch_if: branch_if_mut,
                            branch_else: branch_else_mut,
                        }),
                    ));

                    let exit_node = *self.exit_node().borrow();
                    self.add_edge(if_branch_exit, exit_node, EdgeType::Forward);
                    self.add_edge(else_branch_exit, exit_node, EdgeType::Forward);
                }
                IrOpcode::control_loop(ControlLoopFields {
                    first_condition,
                    condition,
                    body,
                    flip_if,
                    pre_body,
                }) => {
                    let new_var_map: BTreeMap<_, _> = step
                        .try_borrow()?
                        .globally_scoped_variables()?
                        .map(|global_var| (global_var, RcVar::new_empty()))
                        .collect();
                    if do_ssa {
                        additional_opcodes.push((
                            i,
                            new_var_map
                                .iter()
                                .map(|(global_var, new_var)| {
                                    let (var_access, accessed_var) = variable_maps
                                        .use_current_ssa_for_global::<_, _, _, HQResult<_>>(
                                            global_var,
                                            |ssa_var| {
                                                Ok((
                                                    IrOpcode::data_variable(DataVariableFields {
                                                        var: RefCell::new(ssa_var.clone()),
                                                        local_read: RefCell::new(true),
                                                    }),
                                                    ssa_var.clone(),
                                                ))
                                            },
                                            |arg_index, arg_var| {
                                                Ok((
                                                    IrOpcode::procedures_argument(
                                                        ProceduresArgumentFields {
                                                            index: arg_index,
                                                            arg_var: arg_var.clone(),
                                                            in_warped: true,
                                                            arg_vars: Rc::clone(
                                                                &maybe_proc_context
                                                                    .as_ref()
                                                                    .ok_or_else(|| {
                                                                        make_hq_bug!(
                                                                            "tried to access proc \
                                                                             argument when proc \
                                                                             context was None"
                                                                        )
                                                                    })?
                                                                    .arg_vars,
                                                            ),
                                                        },
                                                    ),
                                                    arg_var.clone(),
                                                ))
                                            },
                                            || {
                                                Ok((
                                                    IrOpcode::data_variable(DataVariableFields {
                                                        var: RefCell::new(global_var.clone()),
                                                        local_read: RefCell::new(false),
                                                    }),
                                                    global_var.clone(),
                                                ))
                                            },
                                        )?;
                                    let push_node = self.add_node(Some(StackOperation::Push(
                                        VarTarget::Var(accessed_var),
                                    )));
                                    let pop_node = self.add_node(Some(StackOperation::Pop(
                                        VarTarget::Var(new_var.clone()),
                                    )));
                                    let exit_node = *self.exit_node().borrow();
                                    self.add_edge(exit_node, push_node, EdgeType::Forward);
                                    self.add_edge(push_node, pop_node, EdgeType::Forward);
                                    *self.exit_node().borrow_mut() = pop_node;
                                    variable_maps
                                        .ssa
                                        .insert(global_var.clone(), new_var.clone());
                                    Ok([
                                        var_access,
                                        IrOpcode::data_setvariableto(DataSetvariabletoFields {
                                            var: RefCell::new(new_var.clone()),
                                            local_write: RefCell::new(true),
                                            first_write: RefCell::new(true),
                                        }),
                                    ])
                                })
                                .collect::<HQResult<Vec<_>>>()?
                                .into_iter()
                                .flatten()
                                .collect(),
                        ));
                    }
                    if let Some(first_condition_step) = first_condition {
                        let first_condition_mut =
                            Rc::new(Rc::unwrap_or_clone(Rc::clone(first_condition_step)));
                        graphs.insert(
                            first_condition_mut.try_borrow()?.id().into(),
                            MaybeGraph::Started,
                        );
                        self.visit_step(
                            Rc::clone(&first_condition_mut),
                            // we don't need to keep track of this map as no variables should be set in the first condition
                            &mut variable_maps.clone(),
                            graphs,
                            next_steps,
                            do_ssa,
                        )?;
                        graphs.insert(
                            first_condition_mut.try_borrow()?.id().into(),
                            MaybeGraph::Inlined,
                        );
                        let drop_node = self.add_node(Some(StackOperation::Drop)); // we consume the top item on the type stack as an i32.eqz
                        let end_node = *self.exit_node().borrow();
                        self.add_edge(end_node, drop_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = drop_node;
                        let first_cond_exit = drop_node;

                        let header_node = self.add_node(None);
                        self.add_edge(first_cond_exit, header_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = header_node;

                        let pre_body_mut = pre_body
                            .as_ref()
                            .map(|pre_body_real| -> HQResult<_> {
                                let pre_body_mut =
                                    Rc::new(Rc::unwrap_or_clone(Rc::clone(pre_body_real)));
                                let mut pre_body_variable_maps = variable_maps.clone();
                                graphs.insert(
                                    pre_body_mut.try_borrow()?.id().into(),
                                    MaybeGraph::Started,
                                );
                                self.visit_step(
                                    Rc::clone(&pre_body_mut),
                                    &mut pre_body_variable_maps,
                                    graphs,
                                    next_steps,
                                    do_ssa,
                                )?;
                                graphs.insert(
                                    pre_body_mut.try_borrow()?.id().into(),
                                    MaybeGraph::Inlined,
                                );
                                Ok(pre_body_mut)
                            })
                            .transpose()?;

                        let body_mut = Rc::new(Rc::unwrap_or_clone(Rc::clone(body)));
                        let mut body_variable_maps = variable_maps.clone();
                        graphs.insert(body_mut.try_borrow()?.id().into(), MaybeGraph::Started);
                        self.visit_step(
                            Rc::clone(&body_mut),
                            &mut body_variable_maps,
                            graphs,
                            next_steps,
                            do_ssa,
                        )?;
                        graphs.insert(body_mut.try_borrow()?.id().into(), MaybeGraph::Inlined);

                        let condition_mut: InlinedStep =
                            Rc::new(Rc::unwrap_or_clone(Rc::clone(condition)));
                        graphs.insert(condition_mut.try_borrow()?.id().into(), MaybeGraph::Started);
                        self.visit_step(
                            Rc::clone(&condition_mut),
                            &mut body_variable_maps,
                            graphs,
                            next_steps,
                            do_ssa,
                        )?;
                        graphs.insert(condition_mut.try_borrow()?.id().into(), MaybeGraph::Inlined);
                        let drop_node = self.add_node(Some(StackOperation::Drop));
                        let end_node = *self.exit_node().borrow();
                        self.add_edge(end_node, drop_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = drop_node;
                        let mut cond_exit = drop_node;

                        if do_ssa {
                            self.ssa_phi(
                                vec![(&condition_mut, body_variable_maps.ssa, &mut cond_exit)],
                                variable_maps,
                                &new_var_map,
                            )?;
                        }

                        opcode_replacements.push((
                            i,
                            IrOpcode::control_loop(ControlLoopFields {
                                body: body_mut,
                                first_condition: Some(first_condition_mut),
                                flip_if: *flip_if,
                                condition: condition_mut,
                                pre_body: pre_body_mut,
                            }),
                        ));

                        self.add_edge(cond_exit, header_node, EdgeType::BackLink);
                        *self.exit_node().borrow_mut() = cond_exit;
                    } else {
                        let header_node = self.add_node(None);
                        let last_node = *self.exit_node().borrow();
                        self.add_edge(last_node, header_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = header_node;

                        let condition_mut = Rc::new(Rc::unwrap_or_clone(Rc::clone(condition)));
                        let mut body_variable_maps = variable_maps.clone();
                        graphs.insert(condition_mut.try_borrow()?.id().into(), MaybeGraph::Started);
                        self.visit_step(
                            Rc::clone(&condition_mut),
                            &mut body_variable_maps,
                            graphs,
                            next_steps,
                            do_ssa,
                        )?;
                        graphs.insert(condition_mut.try_borrow()?.id().into(), MaybeGraph::Inlined);
                        let drop_node = self.add_node(Some(StackOperation::Drop));
                        let end_node = *self.exit_node().borrow();
                        self.add_edge(end_node, drop_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = drop_node;

                        let pre_body_mut = pre_body
                            .as_ref()
                            .map(|pre_body_real| -> HQResult<_> {
                                let pre_body_mut =
                                    Rc::new(Rc::unwrap_or_clone(Rc::clone(pre_body_real)));
                                let mut pre_body_variable_maps = variable_maps.clone();
                                graphs.insert(
                                    pre_body_mut.try_borrow()?.id().into(),
                                    MaybeGraph::Started,
                                );
                                self.visit_step(
                                    Rc::clone(&pre_body_mut),
                                    &mut pre_body_variable_maps,
                                    graphs,
                                    next_steps,
                                    do_ssa,
                                )?;
                                graphs.insert(
                                    pre_body_mut.try_borrow()?.id().into(),
                                    MaybeGraph::Inlined,
                                );
                                Ok(pre_body_mut)
                            })
                            .transpose()?;

                        let body_mut = Rc::new(Rc::unwrap_or_clone(Rc::clone(body)));
                        graphs.insert(body_mut.try_borrow()?.id().into(), MaybeGraph::Started);
                        self.visit_step(
                            Rc::clone(&body_mut),
                            &mut body_variable_maps,
                            graphs,
                            next_steps,
                            do_ssa,
                        )?;
                        graphs.insert(body_mut.try_borrow()?.id().into(), MaybeGraph::Inlined);
                        let mut body_exit = *self.exit_node().borrow();

                        if do_ssa {
                            self.ssa_phi(
                                vec![(&body_mut, body_variable_maps.ssa, &mut body_exit)],
                                variable_maps,
                                &new_var_map,
                            )?;
                        }

                        self.add_edge(body_exit, header_node, EdgeType::BackLink);
                        *self.exit_node().borrow_mut() = body_exit;

                        opcode_replacements.push((
                            i,
                            IrOpcode::control_loop(ControlLoopFields {
                                flip_if: *flip_if,
                                body: body_mut,
                                first_condition: None,
                                condition: condition_mut,
                                pre_body: pre_body_mut,
                            }),
                        ));
                    }
                    let loop_exit = *self.exit_node().borrow();
                    let new_node = self.add_node(None);
                    self.add_edge(loop_exit, new_node, EdgeType::Forward);
                    *self.exit_node().borrow_mut() = new_node;
                }
                IrOpcode::procedures_argument(ProceduresArgumentFields { arg_var, .. }) => {
                    // crate::log("found proc argument.");
                    self.add_node_at_end(Some(StackOperation::Push(VarTarget::Var(
                        arg_var.clone(),
                    ))));
                }
                IrOpcode::procedures_call_warp(ProceduresCallWarpFields { proc }) => {
                    // crate::log!("found proc call. type stack: {type_stack:?}");
                    let Some(warped_specific_proc) = &*proc.warped_specific_proc() else {
                        hq_bug!("tried to call_warp with no warped proc")
                    };
                    for arg_var in warped_specific_proc
                        .arg_vars()
                        .try_borrow()?
                        .iter()
                        .rev()
                        .cloned()
                    {
                        let pop_node =
                            self.add_node(Some(StackOperation::Pop(VarTarget::Var(arg_var))));
                        let last_node = *self.exit_node().borrow();
                        self.add_edge(last_node, pop_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = pop_node;
                    }
                    for ret_var in warped_specific_proc
                        .return_vars()
                        .try_borrow()?
                        .iter()
                        .cloned()
                    {
                        let push_node =
                            self.add_node(Some(StackOperation::Push(VarTarget::Var(ret_var))));
                        let last_node = *self.exit_node().borrow();
                        self.add_edge(last_node, push_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = push_node;
                    }
                }
                IrOpcode::procedures_call_nonwarp(ProceduresCallNonwarpFields {
                    proc,
                    next_step,
                }) => {
                    // crate::log!("found proc call. type stack: {type_stack:?}");
                    let Some(nonwarped_specific_proc) = &*proc.nonwarped_specific_proc() else {
                        hq_bug!("tried to call_nonwarp with no non-warped proc")
                    };
                    for arg_var in nonwarped_specific_proc
                        .arg_vars()
                        .try_borrow()?
                        .iter()
                        .rev()
                        .cloned()
                    {
                        let pop_node =
                            self.add_node(Some(StackOperation::Pop(VarTarget::Var(arg_var))));
                        let last_node = *self.exit_node().borrow();
                        self.add_edge(last_node, pop_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = pop_node;
                    }
                    next_steps.push(*next_step);
                    should_propagate_ssa = true;
                    break 'opcode_loop;
                    // crate::log!("type stack after proc call: {type_stack:?}");
                }
                IrOpcode::hq_yield(HqYieldFields { mode }) => match mode {
                    YieldMode::Inline(step) => {
                        // crate::log("found inline step to visit");
                        let step_mut = Rc::new(Rc::unwrap_or_clone(Rc::clone(step)));
                        graphs.insert(step_mut.try_borrow()?.id().into(), MaybeGraph::Started);
                        self.visit_step(
                            Rc::clone(&step_mut),
                            variable_maps,
                            graphs,
                            next_steps,
                            do_ssa,
                        )?;
                        graphs.insert(step_mut.try_borrow()?.id().into(), MaybeGraph::Inlined);
                    }
                    YieldMode::None | YieldMode::Return => {
                        // crate::log("found a yield::none, breaking");
                        should_propagate_ssa = true;
                        step_ended_on_stop = true;
                        break 'opcode_loop;
                    }
                    YieldMode::Schedule(_) => (), // handled at bottom of loop
                },
                _ => {
                    let opcode_node = self.add_node(Some(StackOperation::Opcode(opcode.clone())));
                    let last_node = *self.exit_node().borrow();
                    self.add_edge(last_node, opcode_node, EdgeType::Forward);
                    *self.exit_node().borrow_mut() = opcode_node;
                }
            }
            if let Some(next_step) = opcode.yields_to_next_step() {
                next_steps.push(next_step);
                should_propagate_ssa = true;
                break 'opcode_loop;
            }
        }

        if (step.try_borrow()?.used_non_inline() || step_ended_on_stop)
            && step.try_borrow()?.context().warp
            && let Some(proc_context) = maybe_proc_context.as_ref()
        {
            // crate::log!(
            //     "found spare items on type stack at end of step visit, in warped proc: \
            //      {type_stack:?}"
            // );
            // crate::log!(
            //     "return vars: {}",
            //     proc_context.return_vars().try_borrow()?.len()
            // );
            for ret_var in (*proc_context.ret_vars).borrow().iter().rev().cloned() {
                let pop_node = self.add_node(Some(StackOperation::Pop(VarTarget::Var(ret_var))));
                let last_node = *self.exit_node().borrow();
                self.add_edge(last_node, pop_node, EdgeType::Forward);
                *self.exit_node().borrow_mut() = pop_node;
            }
        }

        {
            let mut step_mut = step.try_borrow_mut()?;
            let opcodes = step_mut.opcodes_mut();

            for (index, opcode_replacement) in opcode_replacements {
                *opcodes
                    .get_mut(index)
                    .ok_or_else(|| make_hq_bug!("opcode index out of bounds"))? =
                    opcode_replacement;
            }

            for (index, additional_opcodes) in additional_opcodes.into_iter().rev() {
                opcodes.splice(index..index, additional_opcodes);
            }
        }

        if should_propagate_ssa {
            let post_yield = {
                if let Some(last_op) = step.try_borrow()?.opcodes().last()
                    && matches!(
                        last_op,
                        IrOpcode::hq_yield(_) | IrOpcode::procedures_call_nonwarp(_)
                    )
                {
                    true
                } else {
                    false
                }
            };
            let mut step_mut = step.try_borrow_mut()?;
            let opcodes = step_mut.opcodes_mut();
            let yield_op = if post_yield {
                #[expect(clippy::unwrap_used, reason = "guaranteed last element")]
                Some(opcodes.pop().unwrap())
            } else {
                None
            };
            for (global_var, ssa_var) in &variable_maps.ssa {
                let push_node =
                    self.add_node(Some(StackOperation::Push(VarTarget::Var(ssa_var.clone()))));
                let pop_node = self.add_node(Some(StackOperation::Pop(VarTarget::Var(
                    global_var.clone(),
                ))));
                let last_node = *self.exit_node().borrow();
                self.add_edge(last_node, push_node, EdgeType::Forward);
                self.add_edge(push_node, pop_node, EdgeType::Forward);
                *self.exit_node().borrow_mut() = pop_node;
                // crate::log("adding outer variable writes");
                opcodes.extend([
                    IrOpcode::data_variable(DataVariableFields {
                        var: RefCell::new(ssa_var.clone()),
                        local_read: RefCell::new(true),
                    }),
                    IrOpcode::data_setvariableto(DataSetvariabletoFields {
                        var: RefCell::new(global_var.clone()),
                        local_write: RefCell::new(false),
                        first_write: RefCell::new(false),
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
        mut ssa_blocks: Vec<(&InlinedStep, BTreeMap<RcVar, RcVar>, &mut NodeIndex)>,
        variable_maps: &mut VariableMaps,
        ssa_write_map: &BTreeMap<RcVar, RcVar>,
    ) -> HQResult<()> {
        let mut lower_ssas: BTreeMap<_, BTreeMap<_, _>> = BTreeMap::new();
        let write_map_was_empty = ssa_write_map.is_empty();

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
                .unwrap_or_else(RcVar::new_empty);
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
            let mut block_mut = block.try_borrow_mut()?;
            let opcodes = block_mut.opcodes_mut();
            if !matches!(
                opcodes.last(),
                Some(IrOpcode::hq_yield(HqYieldFields {
                    mode: YieldMode::Return,
                }))
            ) {
                opcodes.reserve_exact(var_writes.len() * 2);
                for ((var_read, read_local), var_write) in var_writes {
                    let push_node =
                        self.add_node(Some(StackOperation::Push(VarTarget::Var(var_read.clone()))));
                    let pop_node =
                        self.add_node(Some(StackOperation::Pop(VarTarget::Var(var_write.clone()))));
                    self.add_edge(***block_exit, push_node, EdgeType::Forward);
                    self.add_edge(push_node, pop_node, EdgeType::Forward);
                    ***block_exit = pop_node;

                    opcodes.append(&mut vec![
                        IrOpcode::data_variable(DataVariableFields {
                            var: RefCell::new(var_read.clone()),
                            local_read: RefCell::new(*read_local),
                        }),
                        IrOpcode::data_setvariableto(DataSetvariabletoFields {
                            var: RefCell::new(var_write.clone()),
                            local_write: RefCell::new(true),
                            // TODO: might this actually be the first write?
                            first_write: RefCell::new(write_map_was_empty),
                        }),
                    ]);
                }
            }
        }

        Ok(())
    }
}

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
            // crate::log!("finished graph for step {}", next_step.id());
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
    // crate::log("visiting procedure");
    if let Some(warped_specific_proc) = &*proc.warped_specific_proc()
        && let PartialStep::Finished(step_index) = warped_specific_proc.first_step()?.clone()
    {
        // crate::log!("visiting procedure's step: {}", step.id());
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
        // crate::log!("visiting procedure's step: {}", step.id());
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
    // crate::log("splitting variables and making graphs");
    let mut graphs = BTreeMap::new();
    for (_, target) in project.targets().borrow().iter() {
        for (_, proc) in target.procedures()?.iter() {
            visit_procedure(proc, &mut graphs, project, do_ssa)?;
        }
    }
    for thread in project.threads().try_borrow()?.iter() {
        let step_index = thread.first_step();
        // crate::log!("visiting (recursively) step from thread: {}", step.id());
        visit_step_recursively(step_index, project, &mut graphs, &BTreeMap::new(), do_ssa)?;
    }
    // crate::log("finished splitting variables and making graphs");
    Ok(graphs)
}

fn evaluate_type_stack(
    type_stack: &Rc<TypeStack>,
    stack_op: &StackOperation,
    changed_vars: &mut BTreeSet<VarTarget>,
    type_convergence: VarTypeConvergence,
) -> HQResult<Rc<TypeStack>> {
    // crate::log!("type stack: {type_stack:?}");
    Ok(match stack_op {
        StackOperation::Opcode(op) => {
            let mut local_stack = Rc::clone(type_stack);
            let inputs_len = op.acceptable_inputs()?.len();
            // crate::log!("opcode: {op}");
            // crate::log!("stack length: {}, inputs len: {}", stack.len(), inputs_len);
            // crate::log!("stack: {:?}", stack);
            let inputs: Rc<[_]> = local_stack
                .by_ref()
                .take(inputs_len)
                .map(|ty: IrType| if ty.is_none() { IrType::Any } else { ty })
                .collect::<Box<[_]>>()
                .into_iter()
                .rev()
                .collect();
            hq_assert!(
                inputs.len() == inputs_len,
                "can't pop {} elements from stack of length {}",
                inputs_len,
                inputs.len()
            );
            let output = op.output_type(inputs)?;
            match output {
                ReturnType::Singleton(ty) => {
                    local_stack = Rc::new(TypeStack::Cons(ty, Rc::clone(&local_stack)));
                }
                ReturnType::MultiValue(tys) => {
                    for ty in tys.iter().copied() {
                        local_stack = Rc::new(TypeStack::Cons(ty, local_stack));
                    }
                }
                ReturnType::None => (),
            }
            local_stack
        }
        StackOperation::Drop => {
            let TypeStack::Cons(_, tail) = &**type_stack else {
                hq_bug!("can't drop from empty stack");
            };
            Rc::clone(tail)
        }
        StackOperation::Push(var_target) => Rc::new(TypeStack::Cons(
            *var_target.possible_types(),
            Rc::clone(type_stack),
        )),
        StackOperation::Pop(target) => {
            let TypeStack::Cons(head, tail) = &**type_stack else {
                hq_bug!("can't pop from empty stack");
            };
            let expanded_type = match type_convergence {
                VarTypeConvergence::Any => IrType::Any,
                VarTypeConvergence::Base => head.base_types().fold(IrType::none(), IrType::or),
                VarTypeConvergence::Tight => *head,
            };
            if !target.possible_types().contains(expanded_type) {
                changed_vars.insert(target.clone());
            }
            target.add_type(expanded_type);
            Rc::clone(tail)
        }
    })
}

#[derive(Debug, Clone)]
enum TypeStack {
    Nil,
    Cons(IrType, Rc<Self>),
}

impl Iterator for Rc<TypeStack> {
    type Item = IrType;

    fn next(&mut self) -> Option<Self::Item> {
        if let TypeStack::Cons(ty, tail) = &*self.clone() {
            *self = Self::clone(tail);
            Some(*ty)
        } else {
            None
        }
    }
}

#[derive(Clone, Debug)]
struct DFSQueueItem {
    pub edge: EdgeIndex,
    pub type_stack: Rc<TypeStack>,
}

fn visit_graph(
    graph: &VarGraph,
    changed_vars: &mut BTreeSet<VarTarget>,
    type_convergence: VarTypeConvergence,
) -> HQResult<()> {
    let inner_graph = graph.graph().try_borrow()?;
    let mut dfs_queue = vec![DFSQueueItem {
        edge: EdgeIndex::new(0), // this assumes that the first node is empty and has a single forward edge. TODO: is this the case?
        type_stack: Rc::new(TypeStack::Nil),
    }];
    let mut joining_edges: BTreeMap<EdgeIndex, Rc<RefCell<u32>>> = BTreeMap::default();
    while let Some(DFSQueueItem { edge, type_stack }) = dfs_queue.pop() {
        let should_wait_for_all_incoming_edges =
            if let Entry::Occupied(mut waiting_to_join) = joining_edges.entry(edge) {
                let waiting = *waiting_to_join.get().try_borrow()? > 1;
                *waiting_to_join.get_mut().try_borrow_mut()? -= 1;
                waiting_to_join.remove();
                if waiting {
                    // we're still waiting for some incoming edges to be visited before we move on
                    continue;
                }
                // we've already previously waited for incoming edges, and we've now seen them all
                false
            } else {
                true
            };
        let node = inner_graph
            .edge_endpoints(edge)
            .ok_or_else(|| make_hq_bug!("couldn't find edge in graph"))?
            .1;
        let new_stack = if let Some(Some(stack_op)) = inner_graph.node_weight(node) {
            evaluate_type_stack(&type_stack, stack_op, changed_vars, type_convergence)?
        } else {
            Rc::clone(&type_stack)
        };
        // crate::log!("edge: {edge:?}, node: {node:?}");
        let can_continue = if should_wait_for_all_incoming_edges {
            let joining_this = Rc::new(RefCell::new(0));
            for in_edge in inner_graph
                .edges_directed(node, EdgeIn)
                .filter(|e| e.id() != edge && e.weight() == &EdgeType::Forward)
            {
                *joining_this.try_borrow_mut()? += 1;
                hq_assert!(
                    !joining_edges.contains_key(&in_edge.id()),
                    "tried to add duplicate edge {:?} to joining_edges",
                    in_edge
                );
                joining_edges.insert(in_edge.id(), Rc::clone(&joining_this));
            }
            *joining_this.try_borrow()? == 0
        } else {
            true
        };
        if can_continue {
            for for_edge in inner_graph
                .edges_directed(node, EdgeOut)
                .filter(|e| e.weight() == &EdgeType::Forward)
            {
                dfs_queue.push(DFSQueueItem {
                    edge: for_edge.id(),
                    type_stack: Rc::clone(&new_stack),
                });
            }
        }
        // crate::log!("dfs queue: {dfs_queue:?}");
        // crate::log!("joining_edges: {joining_edges:?}");
    }
    // crate::log("ok.");
    Ok(())
}

fn iterate_graphs<'a, I>(graphs: &'a I, type_convergence: VarTypeConvergence) -> HQResult<()>
where
    I: Iterator<Item = (&'a VarGraph, Box<str>)> + Clone,
{
    loop {
        let mut changed_vars: BTreeSet<VarTarget> = BTreeSet::new();
        for graph in graphs.clone() {
            crate::log!("visiting graph for step {}", graph.1);
            // use petgraph::dot::Dot;
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
            visit_graph(graph.0, &mut changed_vars, type_convergence)?;
        }
        // this must eventually converge, because the sequence of types for each variable at each
        // iteration is increasing, but the set of types is finite
        if changed_vars.is_empty() {
            break;
        }
    }
    Ok(())
}

fn box_proc_returns<S>(
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

/// A token type that cannot be instantiated from anywhere else (since the field is private)
/// - used as proof that we've carried out these optimisations.
#[derive(Copy, Clone)]
pub struct SSAToken(PhantomData<()>);

pub fn optimise_variables(
    project: &Rc<IrProject>,
    type_convergence: VarTypeConvergence,
    do_ssa: Switch,
) -> HQResult<SSAToken> {
    // crate::log("carrying out variable optimisation");
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
    // crate::log!("graphs num: {}", graphs.len());
    iterate_graphs(
        &graphs.iter().map(|(s, g)| (*g, (*s).clone())),
        type_convergence,
    )?;
    crate::log("finished iterating graphs");
    for step in project.steps().try_borrow()?.iter() {
        crate::log!("inserting casts for step {}", step.try_borrow()?.id());
        insert_casts(step.try_borrow_mut()?.opcodes_mut(), false, true)?;
        crate::log!(
            "finished inserting casts for step {}",
            step.try_borrow()?.id()
        );
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
