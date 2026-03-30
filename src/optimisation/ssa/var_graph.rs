use core::hash::{Hash, Hasher};
use core::ops::Deref;

use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::stable_graph::StableDiGraph;

use crate::instructions::{
    ControlIfElseFields, ControlLoopFields, DataAddtolistFields, DataInsertatlistFields,
    DataItemoflistFields, DataReplaceitemoflistFields, DataSetvariabletoFields,
    DataTeevariableFields, DataVariableFields, HqYieldFields, IrOpcode, ProceduresArgumentFields,
    ProceduresCallNonwarpFields, ProceduresCallWarpFields, YieldMode,
};
use crate::ir::{InlinedStep, ProcContext, RcList, RcVar, Step, StepIndex, Type as IrType};
use crate::prelude::*;

#[derive(Clone, Debug)]
pub enum StackOperation {
    Drop,
    Push(VarTarget),
    Pop(VarTarget),
    /// this shouldn't be anything that branches or mutates variables in any way,
    /// i.e. not loop, if/else, yield, set variable, etc
    Opcode(IrOpcode),
}

#[derive(Clone, Debug, PartialOrd, PartialEq, Ord, Eq, Hash)]
pub enum VarTarget {
    Var(RcVar),
    List(RcList),
}

impl VarTarget {
    pub fn possible_types(&self) -> core::cell::Ref<'_, IrType> {
        match self {
            Self::Var(var) => var.possible_types(),
            Self::List(list) => list.possible_types(),
        }
    }

    pub fn add_type(&self, ty: IrType) {
        match self {
            Self::Var(var) => var.add_type(ty),
            Self::List(list) => list.add_type(ty),
        }
    }
}

pub enum MaybeGraph {
    Started,
    Inlined,
    Finished(VarGraph),
}

#[derive(Clone)]
pub struct VariableMaps<'a> {
    /// global variable -> local variable
    pub ssa: BTreeMap<RcVar, RcVar>,
    /// global variable -> (arg index, arg variable)
    pub proc_args: &'a BTreeMap<RcVar, (usize, RcVar)>,
}

impl<'a> VariableMaps<'a> {
    pub const fn new_with_proc_args(proc_args: &'a BTreeMap<RcVar, (usize, RcVar)>) -> Self {
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

/// Is this edge a backlink or not?
///
/// We could calculate this dynamically during the reduction stage but seeing as we can know this
/// when constructing the graph, we might as well do it now.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EdgeType {
    Forward,
    BackLink,
}

pub type NodeWeight = Option<StackOperation>;

#[derive(Debug)]
pub struct BaseVarGraph {
    pub graph: RefCell<StableDiGraph<NodeWeight, EdgeType>>,
    pub exit_node: RefCell<NodeIndex>,
}

#[derive(Clone, Debug)]
pub struct VarGraph(Rc<BaseVarGraph>);

impl Hash for VarGraph {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Rc::as_ptr(&self.0).hash(state);
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

    pub fn graph(&self) -> &RefCell<StableDiGraph<NodeWeight, EdgeType>> {
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
    pub fn visit_step<S>(
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
        if let Some(MaybeGraph::Inlined | MaybeGraph::Finished(_)) =
            graphs.get(step.try_borrow()?.id())
        {
            // we've already visited this step.
            return Ok(());
        }

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
            // let's just assume that input types match up.
            #[expect(clippy::wildcard_enum_match_arm, reason = "too many variants to match")]
            match opcode {
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var,
                    local_write: locality,
                    first_write,
                }) => {
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
                    if *local_read.try_borrow()? {
                        self.add_node_at_end(Some(StackOperation::Push(VarTarget::Var(
                            var.try_borrow()?.clone(),
                        ))));
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
                IrOpcode::control_loop(fields) => {
                    self.loop_ssa(
                        &step,
                        variable_maps,
                        graphs,
                        next_steps,
                        &mut opcode_replacements,
                        &mut additional_opcodes,
                        maybe_proc_context.as_ref(),
                        i,
                        fields,
                        do_ssa,
                    )?;
                }
                IrOpcode::procedures_argument(ProceduresArgumentFields { arg_var, .. }) => {
                    self.add_node_at_end(Some(StackOperation::Push(VarTarget::Var(
                        arg_var.clone(),
                    ))));
                }
                IrOpcode::procedures_call_warp(ProceduresCallWarpFields { proc }) => {
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
                }
                IrOpcode::hq_yield(HqYieldFields { mode }) => match mode {
                    YieldMode::Inline(step) => {
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
            for ret_var in (*proc_context.ret_vars).borrow().iter().rev().cloned() {
                let pop_node = self.add_node(Some(StackOperation::Pop(VarTarget::Var(ret_var))));
                let last_node = *self.exit_node().borrow();
                self.add_edge(last_node, pop_node, EdgeType::Forward);
                *self.exit_node().borrow_mut() = pop_node;
            }
        }

        // extra block so the `step_mut` mutable borrow gets dropped before
        // `propagate_ssa` is called
        {
            let mut step_mut = step.try_borrow_mut()?;
            let opcodes = step_mut.opcodes_mut();

            // enact opcode replacements
            for (index, opcode_replacement) in opcode_replacements {
                *opcodes
                    .get_mut(index)
                    .ok_or_else(|| make_hq_bug!("opcode index out of bounds"))? =
                    opcode_replacement;
            }

            // insert additional opcodes
            for (index, additional_opcodes) in additional_opcodes.into_iter().rev() {
                opcodes.splice(index..index, additional_opcodes);
            }
        }

        if should_propagate_ssa {
            self.propagate_ssa(&step, variable_maps)?;
        }
        Ok(())
    }

    fn make_loop_prelude(
        &self,
        variable_maps: &mut VariableMaps,
        maybe_proc_context: Option<&ProcContext>,
        new_var_map: &BTreeMap<RcVar, RcVar>,
    ) -> HQResult<Vec<IrOpcode>> {
        Ok(new_var_map
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
                                IrOpcode::procedures_argument(ProceduresArgumentFields {
                                    index: arg_index,
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
                let push_node =
                    self.add_node(Some(StackOperation::Push(VarTarget::Var(accessed_var))));
                let pop_node =
                    self.add_node(Some(StackOperation::Pop(VarTarget::Var(new_var.clone()))));
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
            .collect())
    }

    fn loop_ssa<S>(
        &mut self,
        step: &S,
        variable_maps: &mut VariableMaps,
        graphs: &mut BTreeMap<Box<str>, MaybeGraph>,
        next_steps: &mut Vec<StepIndex>,
        opcode_replacements: &mut Vec<(usize, IrOpcode)>,
        additional_opcodes: &mut Vec<(usize, Vec<IrOpcode>)>,
        maybe_proc_context: Option<&ProcContext>,
        i: usize,
        ControlLoopFields {
            first_condition,
            condition,
            body,
            flip_if,
            pre_body,
        }: &ControlLoopFields,
        do_ssa: bool,
    ) -> HQResult<()>
    where
        S: Deref<Target = RefCell<Step>>,
    {
        let new_var_map: BTreeMap<_, _> = step
            .try_borrow()?
            .globally_scoped_variables()?
            .map(|global_var| (global_var, RcVar::new_empty()))
            .collect();
        if do_ssa {
            additional_opcodes.push((
                i,
                self.make_loop_prelude(variable_maps, maybe_proc_context, &new_var_map)?,
            ));
        }
        if let Some(first_condition_step) = first_condition {
            let first_condition_mut = Rc::new(Rc::unwrap_or_clone(Rc::clone(first_condition_step)));
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
                    let pre_body_mut = Rc::new(Rc::unwrap_or_clone(Rc::clone(pre_body_real)));
                    let mut pre_body_variable_maps = variable_maps.clone();
                    graphs.insert(pre_body_mut.try_borrow()?.id().into(), MaybeGraph::Started);
                    self.visit_step(
                        Rc::clone(&pre_body_mut),
                        &mut pre_body_variable_maps,
                        graphs,
                        next_steps,
                        do_ssa,
                    )?;
                    graphs.insert(pre_body_mut.try_borrow()?.id().into(), MaybeGraph::Inlined);
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

            let condition_mut: InlinedStep = Rc::new(Rc::unwrap_or_clone(Rc::clone(condition)));
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
                    let pre_body_mut = Rc::new(Rc::unwrap_or_clone(Rc::clone(pre_body_real)));
                    let mut pre_body_variable_maps = variable_maps.clone();
                    graphs.insert(pre_body_mut.try_borrow()?.id().into(), MaybeGraph::Started);
                    self.visit_step(
                        Rc::clone(&pre_body_mut),
                        &mut pre_body_variable_maps,
                        graphs,
                        next_steps,
                        do_ssa,
                    )?;
                    graphs.insert(pre_body_mut.try_borrow()?.id().into(), MaybeGraph::Inlined);
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

        Ok(())
    }

    fn propagate_ssa<S>(&self, step: &S, variable_maps: &VariableMaps) -> HQResult<()>
    where
        S: Deref<Target = RefCell<Step>>,
    {
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
