use super::proc::{procs_from_target, ProcMap};
use super::variable::{variables_from_target, TargetVars};
use super::{Step, Target, Thread};
use crate::instructions::{
    DataSetvariabletoFields, DataTeevariableFields, DataVariableFields, IrOpcode,
    ProceduresArgumentFields,
};
use crate::ir::{used_vars, PartialStep, Proc, ProcContext, RcVar, Type as IrType};
use crate::prelude::*;
use crate::sb3::Sb3Project;
use crate::wasm::WasmFlags;

pub type StepSet = IndexSet<Rc<Step>>;

#[derive(Clone, Debug)]
pub struct IrProject {
    threads: RefCell<Box<[Thread]>>,
    steps: RefCell<StepSet>,
    global_variables: TargetVars,
    targets: RefCell<IndexMap<Box<str>, Rc<Target>>>,
}

impl IrProject {
    pub const fn threads(&self) -> &RefCell<Box<[Thread]>> {
        &self.threads
    }

    pub const fn steps(&self) -> &RefCell<StepSet> {
        &self.steps
    }

    pub const fn targets(&self) -> &RefCell<IndexMap<Box<str>, Rc<Target>>> {
        &self.targets
    }

    pub const fn global_variables(&self) -> &TargetVars {
        &self.global_variables
    }

    pub fn new(global_variables: TargetVars) -> Self {
        Self {
            threads: RefCell::new(Box::new([])),
            steps: RefCell::new(IndexSet::default()),
            global_variables,
            targets: RefCell::new(IndexMap::default()),
        }
    }

    pub fn register_step(&self, step: Rc<Step>) -> HQResult<()> {
        self.steps()
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .insert(step);
        Ok(())
    }

    pub fn try_from_sb3(sb3: &Sb3Project, flags: &WasmFlags) -> HQResult<Rc<Self>> {
        let global_variables = variables_from_target(
            sb3.targets
                .iter()
                .find(|target| target.is_stage)
                .ok_or_else(|| make_hq_bad_proj!("missing stage target"))?,
        );

        let project = Rc::new(IrProject::new(global_variables));

        let (threads_vec, targets): (Vec<_>, Vec<_>) = sb3
            .targets
            .iter()
            .enumerate()
            .map(|(index, target)| {
                let variables = if target.is_stage {
                    BTreeMap::new()
                } else {
                    variables_from_target(target)
                };
                let procedures = RefCell::new(ProcMap::new());
                let ir_target = Rc::new(Target::new(
                    target.is_stage,
                    variables,
                    Rc::downgrade(&project),
                    procedures,
                    index
                        .try_into()
                        .map_err(|_| make_hq_bug!("target index out of bounds"))?,
                ));
                procs_from_target(target, &ir_target)?;
                let blocks = &target.blocks;
                let threads = blocks
                    .iter()
                    .filter_map(|(id, block)| {
                        let thread = Thread::try_from_top_block(
                            block,
                            blocks,
                            Rc::downgrade(&ir_target),
                            &Rc::downgrade(&project),
                            target.comments.clone().iter().any(|(_id, comment)| {
                                matches!(comment.block_id.clone(), Some(d) if &d == id)
                                    && *comment.text.clone() == *"hq-dbg"
                            }),
                            flags,
                        )
                        .transpose()?;
                        Some(thread)
                    })
                    .collect::<HQResult<Box<[_]>>>()?;
                Ok((threads, (target.name.clone(), ir_target)))
            })
            .collect::<HQResult<Box<[_]>>>()?
            .iter()
            .cloned()
            .unzip();
        let threads = threads_vec.into_iter().flatten().collect::<Box<[_]>>();
        *project
            .threads
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))? = threads;
        *project
            .targets
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))? =
            targets.into_iter().collect();
        let global_vars = used_vars(project.global_variables());
        for target in project.targets().try_borrow()?.values() {
            let target_vars = used_vars(target.variables());
            fixup_proc_types(target, &global_vars, &target_vars)?;
        }
        for step in project.steps().try_borrow()?.iter() {
            let target_vars = used_vars(step.context().target()?.variables());
            fixup_proc_calls(step, &global_vars, &target_vars)?;
        }
        Ok(project)
    }
}

/// Add inputs + outputs to procedures corresponding to global/target variables
fn fixup_proc_types(
    target: &Rc<Target>,
    global_vars: &[RcVar],
    target_vars: &[RcVar],
) -> HQResult<()> {
    for procedure in target.procedures()?.values() {
        procedure.context().arg_vars().try_borrow_mut()?.extend(
            (0..(global_vars.len() + target_vars.len()))
                .map(|_| RcVar::new(IrType::none(), crate::sb3::VarVal::Bool(false))),
        );
        procedure.context().return_vars().try_borrow_mut()?.extend(
            (0..(global_vars.len() + target_vars.len()))
                .map(|_| RcVar::new(IrType::none(), crate::sb3::VarVal::Bool(false))),
        );
        if !procedure.context().always_warped() {
            hq_todo!("non-warped procedure for fixup_target_procs")
        }
        let PartialStep::Finished(step) = &*procedure.first_step()? else {
            hq_bug!("found unfinished procedure step")
        };
        {
            let mut opcodes = step.opcodes_mut()?;
            opcodes.extend(
                global_vars
                    .iter()
                    .chain(target_vars)
                    .map(|var| {
                        Ok(IrOpcode::data_variable(DataVariableFields {
                            var: RefCell::new(var.clone()),
                            local_read: RefCell::new(false),
                        }))
                    })
                    .collect::<HQResult<Box<[_]>>>()?,
            );
        }
        fixup_proc_calls(step, global_vars, target_vars)?;
    }

    Ok(())
}

/// Pass variables into procedure calls, and read them on return
fn fixup_proc_calls(step: &Rc<Step>, global_vars: &[RcVar], target_vars: &[RcVar]) -> HQResult<()> {
    let mut call_indices = vec![];
    for (index, opcode) in step.opcodes().try_borrow()?.iter().enumerate() {
        if matches!(opcode, IrOpcode::procedures_call_warp(_)) {
            call_indices.push(index);
        }
    }
    step.opcodes_mut()?
        .reserve_exact(call_indices.len() * (global_vars.len() + target_vars.len()) * 2);
    for call_index in call_indices.iter().rev() {
        #[expect(clippy::range_plus_one, reason = "x+1..=x doesn't make much sense")]
        step.opcodes_mut()?.splice(
            (call_index + 1)..(call_index + 1),
            global_vars.iter().chain(target_vars).rev().map(|var| {
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(var.clone()),
                    local_write: RefCell::new(false),
                })
            }),
        );
        step.opcodes_mut()?.splice(
            call_index..call_index,
            global_vars.iter().chain(target_vars).map(|var| {
                IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(var.clone()),
                    local_read: RefCell::new(false),
                })
            }),
        );
    }

    Ok(())
}

impl fmt::Display for IrProject {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let targets = self
            .targets()
            .borrow()
            .iter()
            .map(|(id, target)| format!(r#""{id}": {target}"#))
            .join(", ");
        let variables = self
            .global_variables()
            .iter()
            .map(|(id, var)| format!(r#""{id}": {var}"#))
            .join(", ");
        let threads = self
            .threads()
            .borrow()
            .iter()
            .map(|thread| format!("{thread}"))
            .join(", ");
        let steps = self
            .steps()
            .borrow()
            .iter()
            .map(|step| format!("{step}"))
            .join(", ");
        write!(
            f,
            r#"{{
        "targets": {{{targets}}},
        "global_variables": {{{variables}}},
        "threads": [{threads}],
        "steps": [{steps}]
    }}"#
        )
    }
}
