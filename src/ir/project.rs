use super::proc::{ProcMap, procs_from_target};
use super::variable::{TargetLists, TargetVars, lists_from_target, variables_from_target};
use super::{Step, Target, Thread};
use crate::instructions::{DataSetvariabletoFields, DataVariableFields, IrOpcode};
use crate::ir::target::IrCostume;
use crate::ir::{PartialStep, RcVar};
use crate::prelude::*;
use crate::sb3::Sb3Project;
use crate::wasm::WasmFlags;

pub type StepSet = IndexSet<Rc<Step>>;

#[derive(Clone, Debug)]
pub struct IrProject {
    threads: RefCell<Box<[Thread]>>,
    steps: RefCell<StepSet>,
    inlined_steps: RefCell<StepSet>,
    global_variables: TargetVars,
    global_lists: TargetLists,
    targets: RefCell<IndexMap<Box<str>, Rc<Target>>>,
}

impl IrProject {
    pub const fn threads(&self) -> &RefCell<Box<[Thread]>> {
        &self.threads
    }

    pub const fn steps(&self) -> &RefCell<StepSet> {
        &self.steps
    }

    pub const fn inlined_steps(&self) -> &RefCell<StepSet> {
        &self.inlined_steps
    }

    pub const fn targets(&self) -> &RefCell<IndexMap<Box<str>, Rc<Target>>> {
        &self.targets
    }

    pub const fn global_variables(&self) -> &TargetVars {
        &self.global_variables
    }

    pub const fn global_lists(&self) -> &TargetLists {
        &self.global_lists
    }

    #[must_use]
    pub fn new(global_variables: TargetVars, global_lists: TargetLists) -> Self {
        Self {
            threads: RefCell::new(Box::new([])),
            steps: RefCell::new(IndexSet::default()),
            inlined_steps: RefCell::new(IndexSet::default()),
            global_variables,
            global_lists,
            targets: RefCell::new(IndexMap::default()),
        }
    }

    pub fn try_from_sb3(sb3: &Sb3Project, flags: &WasmFlags) -> HQResult<Rc<Self>> {
        let global_variables = variables_from_target(
            sb3.targets
                .iter()
                .find(|target| target.is_stage)
                .ok_or_else(|| make_hq_bad_proj!("missing stage target"))?,
        );

        let global_lists = lists_from_target(
            sb3.targets
                .iter()
                .find(|target| target.is_stage)
                .ok_or_else(|| make_hq_bad_proj!("missing stage target"))?,
            flags,
        );

        let project = Rc::new(Self::new(global_variables, global_lists));

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
                let lists = if target.is_stage {
                    BTreeMap::new()
                } else {
                    lists_from_target(target, flags)
                };
                let procedures = RefCell::new(ProcMap::new());
                let costumes = target
                    .costumes
                    .iter()
                    .map(|costume| {
                        IrCostume {
                            name: costume.name.clone(),
                            data_format: costume.data_format,
                            md5ext: costume.md5ext.clone(),
                            //data: load_asset(costume.md5ext.as_str()),
                        }
                    })
                    .collect();
                let ir_target = Rc::new(Target::new(
                    target.is_stage,
                    variables,
                    lists,
                    Rc::downgrade(&project),
                    procedures,
                    index
                        .try_into()
                        .map_err(|_| make_hq_bug!("target index out of bounds"))?,
                    costumes,
                ));
                procs_from_target(target, &ir_target)?;
                let blocks = &target.blocks;
                let threads = blocks
                    .iter()
                    .filter_map(|(id, block)| {
                        let thread = Thread::try_from_top_block(
                            block,
                            blocks,
                            &Rc::clone(&ir_target),
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
        // crate::log("all threads + targets created");
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
        // crate::log!("global_vars: {global_vars:?}");
        for target in project.targets().try_borrow()?.values() {
            fixup_proc_types(target)?;
        }
        for step in project.steps().try_borrow()?.iter() {
            fixup_proc_calls(step)?;
        }
        for step in project.inlined_steps().try_borrow()?.iter() {
            fixup_proc_calls(step)?;
        }
        Ok(project)
    }
}

/// Add inputs + outputs to procedures corresponding to global/target variables
fn fixup_proc_types(target: &Rc<Target>) -> HQResult<()> {
    for procedure in target.procedures()?.values() {
        let Some(ref warped_proc) = *procedure.warped_specific_proc() else {
            continue;
        };

        let PartialStep::Finished(step) = &*warped_proc.first_step()? else {
            continue; // this should hopefully just mean that this procedure is unused.
        };

        let globally_scoped_variables = step.globally_scoped_variables()?;
        let globally_scoped_variables_num = step.globally_scoped_variables_num()?;

        warped_proc
            .arg_vars()
            .try_borrow_mut()?
            .extend((0..globally_scoped_variables_num).map(|_| RcVar::new_empty()));
        warped_proc
            .return_vars()
            .try_borrow_mut()?
            .extend((0..globally_scoped_variables_num).map(|_| RcVar::new_empty()));

        let mut opcodes = step.opcodes_mut()?;

        opcodes.reserve_exact(globally_scoped_variables_num);

        opcodes.extend(globally_scoped_variables.map(|arg_var| {
            IrOpcode::data_variable(DataVariableFields {
                var: RefCell::new(arg_var),
                local_read: RefCell::new(false),
            })
        }));
    }

    Ok(())
}

/// Pass variables into procedure calls, and read them on return
fn fixup_proc_calls(step: &Rc<Step>) -> HQResult<()> {
    let mut call_indices = vec![];
    for (index, opcode) in step.opcodes().try_borrow()?.iter().enumerate() {
        if matches!(opcode, IrOpcode::procedures_call_warp(_)) {
            call_indices.push(index);
        }
    }
    step.opcodes_mut()?
        .reserve_exact(call_indices.len() * step.globally_scoped_variables_num()? * 2);
    for call_index in call_indices.iter().rev() {
        #[expect(clippy::range_plus_one, reason = "x+1..=x doesn't make much sense")]
        step.opcodes_mut()?.splice(
            (call_index + 1)..(call_index + 1),
            step.globally_scoped_variables()?.rev().map(|var| {
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(var),
                    local_write: RefCell::new(false),
                })
            }),
        );
        step.opcodes_mut()?.splice(
            call_index..call_index,
            step.globally_scoped_variables()?.map(|var| {
                IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(var),
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
        let lists = self
            .global_lists()
            .iter()
            .map(|(id, list)| format!(r#""{id}": {list}"#))
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
        let inlined_steps = self
            .inlined_steps()
            .borrow()
            .iter()
            .map(|step| format!("{step}"))
            .join(", ");
        write!(
            f,
            r#"{{
        "targets": {{{targets}}},
        "global_variables": {{{variables}}},
        "global_lists": {{{lists}}},
        "threads": [{threads}],
        "steps": [{steps}],
        "inlined_steps": [{inlined_steps}]
    }}"#
        )
    }
}
