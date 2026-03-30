use alloc::collections::btree_map::Entry;

use petgraph::graph::EdgeIndex;
use petgraph::visit::EdgeRef;
use petgraph::{Incoming as EdgeIn, Outgoing as EdgeOut};

use super::var_graph::{EdgeType, StackOperation, VarGraph, VarTarget};
use crate::ir::{ReturnType, Type as IrType};
use crate::prelude::*;
use crate::wasm::flags::VarTypeConvergence;

fn evaluate_type_stack(
    type_stack: &Rc<TypeStack>,
    stack_op: &StackOperation,
    changed_vars: &mut BTreeSet<VarTarget>,
    type_convergence: VarTypeConvergence,
) -> HQResult<Rc<TypeStack>> {
    Ok(match stack_op {
        StackOperation::Opcode(op) => {
            let mut local_stack = Rc::clone(type_stack);
            let inputs_len = op.acceptable_inputs()?.len();
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
    }
    Ok(())
}

pub fn iterate_graphs<'a, I>(graphs: &'a I, type_convergence: VarTypeConvergence) -> HQResult<()>
where
    I: Iterator<Item = (&'a VarGraph, Box<str>)> + Clone,
{
    loop {
        let mut changed_vars: BTreeSet<VarTarget> = BTreeSet::new();
        for graph in graphs.clone() {
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
