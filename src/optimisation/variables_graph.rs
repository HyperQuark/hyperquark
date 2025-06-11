#![expect(
    clippy::mutable_key_type,
    reason = "implementations of Eq and Ord for RcVar and Step are independent of actual contents"
)]

use crate::instructions::{
    ControlIfElseFields, ControlLoopFields, DataSetvariabletoFields, DataTeevariableFields,
    DataVariableFields, HqYieldFields, IrOpcode, YieldMode,
};
use crate::ir::{IrProject, PartialStep, Proc, RcVar, Step, Type as IrType};
use crate::prelude::*;
use crate::sb3::VarVal;
use core::hash::{Hash, Hasher};

use petgraph::dot::{Config as DotConfig, Dot};
use petgraph::graph::{EdgeIndex, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::{stable_graph::StableDiGraph, Incoming as EdgeIn, Outgoing as EdgeOut};

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

type NodeWeight = Option<(RcVar, Vec<StackElement>)>;

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

    #[expect(clippy::too_many_lines, reason = "difficult to split")]
    fn visit_step<'a>(
        &mut self,
        step: &Rc<Step>,
        outer_variable_map: &'a Option<&'a mut BTreeMap<RcVar, RcVar>>,
        graphs: &mut BTreeMap<Rc<Step>, Option<Self>>,
    ) -> HQResult<()> {
        if graphs.contains_key(step) {
            // we've already visited this step.
            crate::log(format!("visited step {} but it is already visited", step.id()).as_str());
            return Ok(());
        }
        crate::log(format!("visited step {}, not yet visited", step.id()).as_str());
        let mut current_variable_map = const { BTreeMap::new() };
        let mut type_stack: Vec<StackElement> = vec![];
        'opcode_loop: for opcode in step.opcodes().try_borrow()?.iter() {
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
                        // crate::log("found a variable write operation");
                        if *locality.try_borrow()? {
                            // crate::log("variable is already local; skipping");
                            break 'opcode_block;
                        }
                        let new_variable = RcVar::new(IrType::none(), VarVal::Bool(false));
                        current_variable_map
                            .insert(var.try_borrow()?.clone(), new_variable.clone());
                        *var.try_borrow_mut()? = new_variable;
                        *locality.try_borrow_mut()? = true;
                        let new_node =
                            self.add_node(Some((var.try_borrow()?.clone(), type_stack.clone())));
                        let last_node = *self.exit_node().borrow();
                        self.add_edge(last_node, new_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = new_node;
                        type_stack.clear();
                    }
                    IrOpcode::data_variable(DataVariableFields { var, local_read }) => {
                        crate::log("found a variable read operation");
                        if *local_read.try_borrow()? {
                            // crate::log("variable is already local; skipping");
                            break 'opcode_block;
                        }
                        let var_to_swap = if let Some(current_var) =
                            current_variable_map.get(&var.try_borrow()?.clone())
                        {
                            // crate::log("local variable instance found");
                            Some(current_var.clone())
                        } else if let Some(ref outer_var_map) = outer_variable_map
                            && let Some(outer_var) = outer_var_map.get(&var.try_borrow()?.clone())
                        {
                            // crate::log("outer variable instance found");
                            Some(outer_var.clone())
                        } else {
                            None
                        };
                        if let Some(new_var) = var_to_swap {
                            *var.try_borrow_mut()? = new_var;
                            *local_read.try_borrow_mut()? = true;
                        }
                        type_stack.push(StackElement::Var(var.try_borrow()?.clone()));
                    }
                    IrOpcode::control_if_else(ControlIfElseFields {
                        branch_if,
                        branch_else,
                    }) => {
                        // crate::log("found control_if_else");
                        let last_node = *self.exit_node().borrow();
                        self.visit_step(branch_if, &Some(&mut current_variable_map), graphs)?;
                        let if_branch_exit = *self.exit_node().borrow();
                        *self.exit_node().borrow_mut() = last_node;
                        self.visit_step(branch_else, &Some(&mut current_variable_map), graphs)?;
                        let else_branch_exit = *self.exit_node().borrow();
                        *self.exit_node().borrow_mut() = self.add_node(None);
                        let exit_node = *self.exit_node().borrow();
                        self.add_edge(if_branch_exit, exit_node, EdgeType::Forward);
                        self.add_edge(else_branch_exit, exit_node, EdgeType::Forward);
                    }
                    IrOpcode::control_loop(ControlLoopFields {
                        first_condition,
                        condition,
                        body,
                        ..
                    }) => {
                        // crate::log("found control_loop");
                        if let Some(first_condition_step) = first_condition {
                            self.visit_step(
                                first_condition_step,
                                &Some(&mut current_variable_map),
                                graphs,
                            )?;
                            let first_cond_exit = *self.exit_node().borrow();
                            let header_node = self.add_node(None);
                            self.add_edge(first_cond_exit, header_node, EdgeType::Forward);
                            *self.exit_node().borrow_mut() = header_node;
                            self.visit_step(body, &Some(&mut current_variable_map), graphs)?;
                            self.visit_step(condition, outer_variable_map, graphs)?;
                            let cond_exit = *self.exit_node().borrow();
                            self.add_edge(cond_exit, header_node, EdgeType::BackLink);
                            *self.exit_node().borrow_mut() = cond_exit;
                        } else {
                            let header_node = self.add_node(None);
                            let last_node = *self.exit_node().borrow();
                            self.add_edge(last_node, header_node, EdgeType::Forward);
                            *self.exit_node().borrow_mut() = header_node;
                            self.visit_step(condition, outer_variable_map, graphs)?;
                            self.visit_step(body, &Some(&mut current_variable_map), graphs)?;
                            let body_exit = *self.exit_node().borrow();
                            self.add_edge(body_exit, header_node, EdgeType::BackLink);
                            *self.exit_node().borrow_mut() = body_exit;
                        }
                        let loop_exit = *self.exit_node().borrow();
                        let new_node = self.add_node(None);
                        self.add_edge(loop_exit, new_node, EdgeType::Forward);
                        *self.exit_node().borrow_mut() = new_node;
                    }
                    IrOpcode::hq_yield(HqYieldFields { mode }) => match mode {
                        YieldMode::Tail(_) => {
                            hq_todo!("tail-call yield in variable splitting pass")
                        }
                        YieldMode::Inline(step) => {
                            // crate::log("found inline step to visit");
                            self.visit_step(step, &Some(&mut current_variable_map), graphs)?;
                        }
                        YieldMode::None => {
                            // crate::log("found a yield::none, breaking");
                            break 'opcode_loop;
                        }
                        YieldMode::Schedule(step) => {
                            if let Some(rcstep) = step.upgrade() {
                                graphs.insert(Rc::clone(&rcstep), None);
                                let mut graph = Self::new();
                                graph.visit_step(&rcstep, &None, graphs)?;
                                graphs.insert(rcstep, Some(graph));
                            } else {
                                crate::log("couldn't upgrade Weak<Step> in Schedule in variables optimisation pass;\
                    what's going on here?");
                            }
                            break 'opcode_loop;
                        }
                    },
                    _ => {
                        let inputs_len = opcode.acceptable_inputs()?.len();
                        if let Some(output) = opcode
                            .output_type(core::iter::repeat_n(IrType::Any, inputs_len).collect())?
                        {
                            if inputs_len == 0 {
                                type_stack.push(StackElement::Type(output));
                            } else {
                                type_stack.push(StackElement::Opcode(opcode.clone()));
                            }
                        } else {
                            type_stack.clear();
                        }
                    }
                }
            }
        }
        let final_state_changes = generate_changed_state(&current_variable_map, outer_variable_map);
        for (old_var, new_var, _local) in final_state_changes.clone() {
            let this_type_stack = vec![StackElement::Var(old_var)];
            let new_node = self.add_node(Some((new_var, this_type_stack)));
            let last_node = *self.exit_node().borrow();
            self.add_edge(last_node, new_node, EdgeType::Forward);
            *self.exit_node().borrow_mut() = new_node;
        }
        let final_variable_writes = generate_state_backup(final_state_changes);
        let post_yield = if let Some(last_op) = step.opcodes().try_borrow()?.last()
            && matches!(last_op, IrOpcode::hq_yield(_))
        {
            true
        } else {
            false
        };
        // crate::log("adding outer variable writes");
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
}

/// Visits a procedure. Returns the found step, and a boolean indicating if the step was for a warped
/// procedure or not.
fn visit_procedure(
    proc: &Rc<Proc>,
    graphs: &mut BTreeMap<Rc<Step>, Option<VarGraph>>,
) -> HQResult<(Rc<Step>, bool)> {
    Ok(
        if let PartialStep::Finished(step) = proc.warped_first_step()?.clone() {
            crate::log("got warped first step for proc");
            graphs.insert(Rc::clone(&step), None);
            let mut graph = VarGraph::new();
            graph.visit_step(&step, &None, graphs)?;
            graphs.insert(Rc::clone(&step), Some(graph));
            (step, true)
        } else if let PartialStep::Finished(step) = proc.non_warped_first_step()?.clone() {
            crate::log("got non-warped first step for proc");
            graphs.insert(Rc::clone(&step), None);
            let mut graph = VarGraph::new();
            graph.visit_step(&step, &None, graphs)?;
            graphs.insert(Rc::clone(&step), Some(graph));
            (step, false)
        } else {
            hq_bug!("no finished step for procedure, whether warped or not")
        },
    )
}

/// Splits variables at every write site, and ensures that outer/global variables are then written
/// to at the end of a step. This also constructs variable type graphs for each step which can then
/// be analyzed to determine the best types for variables.
fn split_variables_and_make_graphs(
    project: &Rc<IrProject>,
) -> HQResult<BTreeMap<Rc<Step>, Option<VarGraph>>> {
    crate::log("splitting variables");
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
        graphs.insert(Rc::clone(thread.first_step()), None);
        let mut graph = VarGraph::new();
        graph.visit_step(thread.first_step(), &None, &mut graphs)?;
        graphs.insert(Rc::clone(thread.first_step()), Some(graph));
    }
    crate::log("finished splitting variables");
    Ok(graphs)
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

fn evaluate_type_stack(type_stack: &Vec<StackElement>) -> HQResult<IrType> {
    let mut stack = vec![];
    for el in type_stack {
        match el {
            StackElement::Opcode(op) => {
                let inputs_len = op.acceptable_inputs()?.len();
                let inputs: Rc<[_]> = stack.splice((stack.len() - inputs_len).., []).collect();
                if let Some(output) = op.output_type(inputs)? {
                    stack.push(output);
                }
            }
            StackElement::Type(ty) => stack.push(*ty),
            StackElement::Var(var) => stack.push(*var.possible_types()),
        }
    }
    hq_assert!(stack.len() == 1);
    #[expect(clippy::unwrap_used, reason = "Asserted that one element exists")]
    Ok(*stack.first().unwrap())
}

fn visit_node(
    graph: &VarGraph,
    node: NodeIndex,
    changed_vars: &mut BTreeSet<RcVar>,
    stop_at: NodeIndex,
) -> HQResult<()> {
    if let Some(Some((var, type_stack))) = graph.graph().try_borrow()?.node_weight(node) {
        let new_type = evaluate_type_stack(type_stack)?;
        if !var.possible_types().contains(new_type) {
            changed_vars.insert(var.clone());
        }
        var.add_type(new_type);
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
    crate::log("carrying out variable optimisation");
    let graphs = split_variables_and_make_graphs(project)?
        .into_iter()
        .map(|(step, graph)| {
            Ok((
                step,
                graph.ok_or_else(|| make_hq_bug!("found None graph in graph map"))?,
            ))
        })
        .collect::<HQResult<BTreeMap<_, _>>>()?;
    crate::log!("graphs num: {}", graphs.len());
    for graph in graphs.values() {
        crate::log!(
            "{:?}exit node: {:?}",
            Dot::with_config(&*graph.graph().borrow(), &[DotConfig::NodeIndexLabel]),
            *graph.exit_node().borrow()
        );
    }
    iterate_graphs(&graphs.values())?;
    Ok(())
}
