#![expect(
    clippy::mutable_key_type,
    reason = "implementations of Eq and Ord for RcVar and Step are independent of actual contents"
)]

use crate::instructions::{
    ControlIfElseFields, ControlLoopFields, DataSetvariabletoFields, DataTeevariableFields,
    DataVariableFields, HqYieldFields, IrOpcode, ProceduresArgumentFields,
    ProceduresCallWarpFields, YieldMode,
};
use crate::ir::{
    IrProject, PartialStep, Proc, RcVar, ReturnType, Step, Type as IrType, insert_casts, used_vars,
};
use crate::prelude::*;
use crate::sb3::VarVal;

use alloc::collections::btree_map::Entry;
use core::convert::identity;
use core::hash::{Hash, Hasher};
use core::mem;

use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::{Incoming as EdgeIn, Outgoing as EdgeOut, stable_graph::StableDiGraph};

#[derive(Clone, Debug)]
enum StackElement {
    Var(RcVar),
    // for things where we can guarantee the output type. e.g. for constants (int, float, string)
    Type(IrType),
    /// this shouldn't be anything that branches or mutates anything in any way,
    /// i.e. not loop, if/else, yield, set variable, etc
    Opcode(IrOpcode),
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

type NodeWeight = Option<(Vec<RcVar>, Vec<StackElement>)>;

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

struct VariableMaps<'a> {
    /// global variable -> local variable
    pub current: BTreeMap<RcVar, RcVar>,
    /// global variable -> local variable, for the immediate outer scope
    pub outer: &'a BTreeMap<RcVar, RcVar>,
    /// global variable -> local variable, for any outer scope
    pub global: &'a BTreeMap<RcVar, RcVar>,
}

impl<'a> VariableMaps<'a> {
    const fn new() -> Self {
        Self {
            current: BTreeMap::new(),
            outer: const { &BTreeMap::new() },
            global: const { &BTreeMap::new() },
        }
    }

    fn elevate(
        &'a self,
        empty_btreemap: &'a mut BTreeMap<RcVar, RcVar>,
        empty_btreemap2: &'a mut BTreeMap<RcVar, RcVar>,
    ) -> Self {
        empty_btreemap.append(&mut self.global.clone());
        empty_btreemap.append(&mut self.outer.clone());
        empty_btreemap2.append(&mut self.outer.clone());
        empty_btreemap2.append(&mut self.current.clone());
        Self {
            current: BTreeMap::new(),
            outer: empty_btreemap2,
            global: empty_btreemap,
        }
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
        let mut opcode_replacements: Vec<(usize, IrOpcode)> = vec![];
        'opcode_loop: for (i, opcode) in step.opcodes().try_borrow()?.iter().enumerate() {
            // crate::log!("opcode: {opcode:?}");
            // let's just assume that input types match up.
            #[expect(clippy::wildcard_enum_match_arm, reason = "too many variants to match")]
            'opcode_block: {
                match opcode {
                    IrOpcode::data_setvariableto(DataSetvariabletoFields {
                        var,
                        local_write: locality,
                    }) => {
                        // crate::log!("found a variable write operation, type stack: {type_stack:?}");
                        let already_local = *locality.try_borrow()?;
                        if !already_local {
                            let new_variable = RcVar::new(IrType::none(), VarVal::Bool(false));
                            variable_maps
                                .current
                                .insert(var.try_borrow()?.clone(), new_variable.clone());
                            *var.try_borrow_mut()? = new_variable;
                            *locality.try_borrow_mut()? = true;
                        }
                        if type_stack.is_empty() {
                            let mut graph = self.graph().borrow_mut();
                            let Some(Some((vars, _))) =
                                graph.node_weight_mut(*self.exit_node().borrow())
                            else {
                                hq_bug!("")
                            };
                            vars.push(var.try_borrow()?.clone());
                        } else if !type_stack.is_empty() {
                            let new_node = self.add_node(Some((
                                vec![var.try_borrow()?.clone()],
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
                        if *locality.try_borrow()? {
                            break 'opcode_block;
                        }
                        let new_variable = RcVar::new(IrType::none(), VarVal::Bool(false));
                        variable_maps
                            .current
                            .insert(var.try_borrow()?.clone(), new_variable.clone());
                        *var.try_borrow_mut()? = new_variable;
                        *locality.try_borrow_mut()? = true;
                        let new_node = self.add_node(Some((
                            vec![var.try_borrow()?.clone()],
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
                            break 'opcode_block;
                        }
                        let var_to_swap = if let Some(current_var) =
                            variable_maps.current.get(&var.try_borrow()?.clone())
                        {
                            // crate::log("local variable instance found");
                            Some(current_var.clone())
                        } else {
                            variable_maps.outer.get(&var.try_borrow()?.clone()).cloned()
                        };
                        if let Some(new_var) = var_to_swap {
                            *var.try_borrow_mut()? = new_var;
                            *local_read.try_borrow_mut()? = true;
                        } else if let Some(proc_context) = maybe_proc_context {
                            let target = step.context().target();
                            let project = step
                                .project()
                                .upgrade()
                                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<IrProject>"))?;
                            let global_vars = used_vars(project.global_variables());
                            let target_vars = used_vars(target.variables());
                            if let Some((var_index, _)) = global_vars
                                .iter()
                                .chain(&target_vars)
                                .find_position(|v| *v == &*var.borrow())
                            {
                                let arg_vars_cell = proc_context.arg_vars();
                                let arg_vars = arg_vars_cell.try_borrow()?;
                                let arg_var = arg_vars
                                    .get(
                                        var_index + arg_vars.len()
                                            - global_vars.len()
                                            - target_vars.len(),
                                    )
                                    .ok_or_else(|| make_hq_bug!("var index out of bounds"))?;
                                // crate::log("inserting var->arg replacement");
                                opcode_replacements.push((
                                    i,
                                    IrOpcode::procedures_argument(ProceduresArgumentFields(
                                        var_index,
                                        arg_var.clone(),
                                    )),
                                ));
                                type_stack.push(StackElement::Var(arg_var.clone()));
                                break 'opcode_block;
                            }
                            hq_bug!("couldn't find variable in global & target vars")
                        }
                        type_stack.push(StackElement::Var(var.try_borrow()?.clone()));
                        // crate::log!("stack: {type_stack:?}");
                    }
                    IrOpcode::control_if_else(ControlIfElseFields {
                        branch_if,
                        branch_else,
                    }) => {
                        // crate::log("found control_if_else");
                        type_stack.clear(); // the top item on the stack should be consumed by control_if_else
                        let last_node = *self.exit_node().borrow();
                        graphs.insert(Rc::clone(branch_if), MaybeGraph::Started);
                        self.visit_step(
                            branch_if,
                            &mut variable_maps.elevate(&mut BTreeMap::new(), &mut BTreeMap::new()),
                            graphs,
                            type_stack,
                            next_steps,
                        )?;
                        graphs.insert(Rc::clone(branch_if), MaybeGraph::Inlined);
                        let if_branch_exit = *self.exit_node().borrow();
                        *self.exit_node().borrow_mut() = last_node;
                        graphs.insert(Rc::clone(branch_else), MaybeGraph::Started);
                        self.visit_step(
                            branch_else,
                            &mut variable_maps.elevate(&mut BTreeMap::new(), &mut BTreeMap::new()),
                            graphs,
                            type_stack,
                            next_steps,
                        )?;
                        graphs.insert(Rc::clone(branch_else), MaybeGraph::Inlined);
                        let else_branch_exit = *self.exit_node().borrow();
                        *self.exit_node().borrow_mut() = self.add_node(None);
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
                        // crate::log("found control_loop");
                        if let Some(first_condition_step) = first_condition {
                            graphs.insert(Rc::clone(first_condition_step), MaybeGraph::Started);
                            self.visit_step(
                                first_condition_step,
                                &mut variable_maps
                                    .elevate(&mut BTreeMap::new(), &mut BTreeMap::new()),
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
                            graphs.insert(Rc::clone(body), MaybeGraph::Started);
                            self.visit_step(
                                body,
                                &mut variable_maps
                                    .elevate(&mut BTreeMap::new(), &mut BTreeMap::new()),
                                graphs,
                                type_stack,
                                next_steps,
                            )?;
                            graphs.insert(Rc::clone(body), MaybeGraph::Inlined);
                            graphs.insert(Rc::clone(condition), MaybeGraph::Started);
                            self.visit_step(
                                condition,
                                &mut variable_maps
                                    .elevate(&mut BTreeMap::new(), &mut BTreeMap::new()),
                                graphs,
                                type_stack,
                                next_steps,
                            )?;
                            graphs.insert(Rc::clone(condition), MaybeGraph::Inlined);
                            type_stack.clear();
                            let cond_exit = *self.exit_node().borrow();
                            self.add_edge(cond_exit, header_node, EdgeType::BackLink);
                            *self.exit_node().borrow_mut() = cond_exit;
                        } else {
                            let header_node = self.add_node(None);
                            let last_node = *self.exit_node().borrow();
                            self.add_edge(last_node, header_node, EdgeType::Forward);
                            *self.exit_node().borrow_mut() = header_node;
                            graphs.insert(Rc::clone(condition), MaybeGraph::Started);
                            self.visit_step(
                                condition,
                                &mut variable_maps
                                    .elevate(&mut BTreeMap::new(), &mut BTreeMap::new()),
                                graphs,
                                type_stack,
                                next_steps,
                            )?;
                            graphs.insert(Rc::clone(condition), MaybeGraph::Inlined);
                            type_stack.clear();
                            graphs.insert(Rc::clone(body), MaybeGraph::Started);
                            self.visit_step(
                                body,
                                &mut variable_maps
                                    .elevate(&mut BTreeMap::new(), &mut BTreeMap::new()),
                                graphs,
                                type_stack,
                                next_steps,
                            )?;
                            graphs.insert(Rc::clone(body), MaybeGraph::Inlined);
                            let body_exit = *self.exit_node().borrow();
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
                            self.visit_step(
                                step,
                                &mut variable_maps
                                    .elevate(&mut BTreeMap::new(), &mut BTreeMap::new()),
                                graphs,
                                type_stack,
                                next_steps,
                            )?;
                            graphs.insert(Rc::clone(step), MaybeGraph::Inlined);
                        }
                        YieldMode::None => {
                            // crate::log("found a yield::none, breaking");
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
                                    type_stack
                                        .extend(outputs.iter().copied().map(StackElement::Type));
                                } else {
                                    type_stack.push(StackElement::Opcode(opcode.clone()));
                                }
                            }
                            ReturnType::None => type_stack.clear(),
                        }
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
                    .collect(),
                mem::take(type_stack),
            )));
            let last_node = *self.exit_node().borrow();
            self.add_edge(last_node, new_node, EdgeType::Forward);
            *self.exit_node().borrow_mut() = new_node;
        }
        for (index, opcode_replacement) in opcode_replacements {
            *step
                .opcodes_mut()?
                .get_mut(index)
                .ok_or_else(|| make_hq_bug!("opcode index out of bounds"))? = opcode_replacement;
        }
        let (post_yield, no_propagation) = if let Some(last_op) =
            step.opcodes().try_borrow()?.last()
            && matches!(last_op, IrOpcode::hq_yield(_))
        {
            (
                true,
                matches!(
                    last_op,
                    IrOpcode::hq_yield(HqYieldFields {
                        mode: YieldMode::Inline(_)
                    })
                ),
            )
        } else {
            (false, false)
        };
        if !no_propagation {
            let final_state_changes = generate_changed_state(variable_maps, post_yield);
            for (old_var, new_var, _local) in final_state_changes.clone() {
                let this_type_stack = vec![StackElement::Var(old_var)];
                let new_node = self.add_node(Some((vec![new_var], this_type_stack)));
                let last_node = *self.exit_node().borrow();
                self.add_edge(last_node, new_node, EdgeType::Forward);
                *self.exit_node().borrow_mut() = new_node;
            }
            let final_variable_writes = generate_state_backup(final_state_changes.into_iter());
            // crate::log("adding outer variable writes");
            if post_yield {
                #[expect(clippy::unwrap_used, reason = "guaranteed last element")]
                let yield_op = step.opcodes_mut()?.pop().unwrap();
                step.opcodes_mut()?.extend(final_variable_writes);
                step.opcodes_mut()?.push(yield_op);
            } else {
                step.opcodes_mut()?.extend(final_variable_writes);
            }
        }
        Ok(())
    }
}

fn visit_step_recursively(
    step: Rc<Step>,
    graphs: &mut BTreeMap<Rc<Step>, MaybeGraph>,
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
                &mut VariableMaps::new(),
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

/// Visits a procedure. Returns the found step, and a boolean indicating if the step was for a warped
/// procedure or not.
fn visit_procedure(proc: &Rc<Proc>, graphs: &mut BTreeMap<Rc<Step>, MaybeGraph>) -> HQResult<()> {
    // crate::log("visiting procedure");
    if let PartialStep::Finished(step) = proc.first_step()?.clone() {
        // crate::log!("visiting procedure's step: {}", step.id());
        visit_step_recursively(step, graphs)?;
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
        visit_step_recursively(step, &mut graphs)?;
    }
    // crate::log("finished splitting variables and making graphs");
    Ok(graphs)
}

/// Generate instructions to write the values of our localised variables to either the local variables
/// belonging to the outer scope, or to the global instance of that variable.
#[expect(
    clippy::mutable_key_type,
    reason = "implementations of Eq and Ord for RcVar are independent of actual contents"
)]
fn generate_changed_state<'a>(
    variable_maps: &'a VariableMaps<'a>,
    post_yield: bool,
) -> Box<[(RcVar, RcVar, bool)]> {
    if post_yield {
        // If we are yielding, we can't rely on outer scopes to propagate all of their variable changes
        // for us. Therefore, we need to consider the `current` variable map, the `outer` variable map
        // *and* the `global` variable map. The inner-scoped variables take priority, since outer-scoped
        // variables are local and will be lost after yielding. All state changes here write to global
        // variables, since we would only write to a local variable if we were propagating up to the
        // immediate outer scope.
        let mut new_map = variable_maps.global.clone();
        new_map.append(&mut variable_maps.outer.clone());
        new_map.append(&mut variable_maps.current.clone());
        new_map
            .iter()
            .map(|(global, local)| (local.clone(), global.clone(), false))
            .collect()
    } else {
        // If we've written to a variable in the current scope, that variable will be present as a key in
        // the `current` variable map, and it may also be present in the `outer` variable map. Any variables
        // that are present in the `outer` variable map but not the `current` variable map will be dealt with
        // in the outer scope, as we are not yielding away here. So, we only need to iterate through the
        // current local variables, and then as we're doing it we can check if there's an outer equivalent.
        variable_maps
            .current
            .iter()
            .map(move |(global, current)| {
                let (to_write, local) = variable_maps
                    .outer
                    .get(global)
                    .map_or((global, false), |outer_var| (outer_var, true));
                (current.clone(), to_write.clone(), local)
            })
            .collect()
    }
}

fn generate_state_backup(
    changed_state: impl Iterator<Item = (RcVar, RcVar, bool)>,
) -> Vec<IrOpcode> {
    changed_state
        .flat_map(|(current, to_write, local)| {
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
        }
    }
    Ok(stack)
}

fn visit_node(
    graph: &VarGraph,
    node: NodeIndex,
    changed_vars: &mut BTreeSet<RcVar>,
    stop_at: NodeIndex,
) -> HQResult<()> {
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
        let mut changed_vars: BTreeSet<RcVar> = BTreeSet::new();
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
        if changed_vars.is_empty() {
            break;
        }
    }
    Ok(())
}

pub fn optimise_variables(project: &Rc<IrProject>) -> HQResult<()> {
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
        crate::log!("inserting casts for step {}", step.id());
        insert_casts(&mut *step.opcodes_mut()?)?;
    }
    Ok(())
}
