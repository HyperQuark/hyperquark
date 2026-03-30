use core::ops::Deref;

use super::proc::{ProcMap, procs_from_target};
use super::variable::{TargetLists, TargetVars, lists_from_target, variables_from_target};
use super::{Step, Target, Thread};
use crate::instructions::{
    DataSetvariabletoFields, DataVariableFields, HqYieldFields, IrOpcode, YieldMode,
};
use crate::ir::step::StepIndex;
use crate::ir::target::IrCostume;
use crate::ir::{PartialStep, RcVar};
use crate::prelude::*;
use crate::sb3::Sb3Project;
use crate::wasm::WasmFlags;

#[derive(Clone, Debug)]
pub struct IrProject {
    threads: RefCell<Box<[Thread]>>,
    steps: RefCell<Vec<RefCell<Step>>>,
    global_variables: TargetVars,
    global_lists: TargetLists,
    broadcasts: Box<[Box<str>]>,
    targets: RefCell<IndexMap<Box<str>, Rc<Target>>>,
    stage_index: usize,
    backdrops: Vec<IrCostume>,
}

impl IrProject {
    pub const fn threads(&self) -> &RefCell<Box<[Thread]>> {
        &self.threads
    }

    pub const fn steps(&self) -> &RefCell<Vec<RefCell<Step>>> {
        &self.steps
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

    pub const fn broadcasts(&self) -> &[Box<str>] {
        &self.broadcasts
    }

    pub const fn stage_index(&self) -> usize {
        self.stage_index
    }

    pub const fn backdrops(&self) -> &Vec<IrCostume> {
        &self.backdrops
    }

    #[must_use]
    pub fn new(
        global_variables: TargetVars,
        global_lists: TargetLists,
        broadcasts: Box<[Box<str>]>,
        stage_index: usize,
        backdrops: Vec<IrCostume>,
    ) -> Self {
        Self {
            threads: RefCell::new(Box::new([])),
            steps: RefCell::new(Vec::new()),
            global_variables,
            global_lists,
            broadcasts,
            targets: RefCell::new(IndexMap::default()),
            stage_index,
            backdrops,
        }
    }

    pub fn new_owned_step(&self, step: Step) -> HQResult<StepIndex> {
        self.steps()
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .push(RefCell::new(step));
        Ok(StepIndex(self.steps().try_borrow()?.len() - 1))
    }

    pub fn try_from_sb3(sb3: &Sb3Project, flags: &WasmFlags) -> HQResult<Rc<Self>> {
        let global_variables = variables_from_target(
            sb3.targets
                .iter()
                .find(|target| target.is_stage)
                .ok_or_else(|| make_hq_bad_proj!("missing stage target"))?,
            &sb3.monitors,
            flags,
        )?;

        let global_lists = lists_from_target(
            sb3.targets
                .iter()
                .find(|target| target.is_stage)
                .ok_or_else(|| make_hq_bad_proj!("missing stage target"))?,
            flags,
        )?;

        let broadcasts = sb3
            .targets
            .iter()
            .flat_map(|target| target.broadcasts.values())
            .cloned()
            .collect();

        let (stage_index, stage_target) = sb3
            .targets
            .iter()
            .find_position(|target| target.is_stage)
            .ok_or_else(|| make_hq_bug!("couldn't find stage target"))?;

        let backdrops: Vec<_> = stage_target
            .costumes
            .iter()
            .map(|costume| IrCostume {
                name: costume.name.clone(),
                data_format: costume.data_format,
                md5ext: costume.md5ext.clone(),
            })
            .collect();

        let project = Rc::new(Self::new(
            global_variables,
            global_lists,
            broadcasts,
            stage_index,
            backdrops,
        ));

        let (threads_vec, targets): (Vec<_>, Vec<_>) = sb3
            .targets
            .iter()
            .enumerate()
            .map(|(index, target)| {
                let variables = if target.is_stage {
                    BTreeMap::new()
                } else {
                    variables_from_target(target, &sb3.monitors, flags)?
                };
                let lists = if target.is_stage {
                    BTreeMap::new()
                } else {
                    lists_from_target(target, flags)?
                };
                let procedures = RefCell::new(ProcMap::new());
                let costumes = target
                    .costumes
                    .iter()
                    .map(|costume| IrCostume {
                        name: costume.name.clone(),
                        data_format: costume.data_format,
                        md5ext: costume.md5ext.clone(),
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
        for target in project.targets().try_borrow()?.values() {
            fixup_proc_types(target)?;
        }
        let fixed_proc_calls = &mut BTreeSet::new();
        for step in project.steps().try_borrow()?.iter() {
            fixup_proc_calls(step, fixed_proc_calls)?;
        }
        Ok(project)
    }
}

fn add_proc_return_vars_before_return<'a, S, I>(
    step: S,
    var_ops: I,
    checked_steps: &mut BTreeSet<Box<str>>,
) -> HQResult<()>
where
    S: Deref<Target = RefCell<Step>>,
    I: IntoIterator<Item = &'a IrOpcode> + Clone,
{
    let mut has_return = false;
    checked_steps.insert(step.try_borrow()?.id().into());
    for (i, opcode) in step.try_borrow()?.opcodes().iter().enumerate() {
        if matches!(
            opcode,
            IrOpcode::hq_yield(HqYieldFields {
                mode: YieldMode::Return,
            })
        ) {
            hq_assert!(
                i == step.try_borrow()?.opcodes().len() - 1,
                "found yield return in non-tail position"
            );
            has_return = true;
        }
        if let Some(inline_steps) = opcode.inline_steps(true) {
            inline_steps.iter().try_for_each(|inline_step| {
                add_proc_return_vars_before_return(
                    Rc::clone(inline_step),
                    var_ops.clone(),
                    checked_steps,
                )
            })?;
        }
    }

    if has_return {
        let mut step_mut = step.try_borrow_mut()?;
        crate::log("borrowing opcodes mutably");
        let opcodes_mut = step_mut.opcodes_mut();
        crate::log("borrowed opcodes mutably");

        opcodes_mut.pop();

        opcodes_mut.extend(var_ops.into_iter().cloned());

        opcodes_mut.push(IrOpcode::hq_yield(HqYieldFields {
            mode: YieldMode::Return,
        }));
    }

    Ok(())
}

/// Add inputs + outputs to procedures corresponding to global/target variables
fn fixup_proc_types(target: &Rc<Target>) -> HQResult<()> {
    let project = target
        .project()
        .upgrade()
        .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<IrProject>"))?;
    let steps = project.steps().try_borrow()?;
    for procedure in target.procedures()?.values() {
        let Some(ref warped_proc) = *procedure.warped_specific_proc() else {
            continue;
        };

        let PartialStep::Finished(step_index) = &*warped_proc.first_step()? else {
            continue; // this should hopefully just mean that this procedure is unused.
        };

        let step = steps
            .get(step_index.0)
            .ok_or_else(|| make_hq_bug!("step index out of bounds"))?;

        let globally_scoped_variables: Box<[_]> =
            step.try_borrow()?.globally_scoped_variables()?.collect();
        let globally_scoped_variables_num = step.try_borrow()?.globally_scoped_variables_num()?;

        warped_proc
            .arg_vars()
            .try_borrow_mut()?
            .extend((0..globally_scoped_variables_num).map(|_| RcVar::new_empty()));
        warped_proc
            .return_vars()
            .try_borrow_mut()?
            .extend((0..globally_scoped_variables_num).map(|_| RcVar::new_empty()));

        let ret_var_ops = globally_scoped_variables
            .iter()
            .cloned()
            .map(|arg_var| {
                IrOpcode::data_variable(DataVariableFields {
                    var: RefCell::new(arg_var),
                    local_read: RefCell::new(false),
                })
            })
            .collect::<Box<[_]>>();

        {
            let mut step_mut = step.try_borrow_mut()?;
            let opcodes = step_mut.opcodes_mut();

            opcodes.reserve_exact(globally_scoped_variables_num);

            opcodes.extend(ret_var_ops.iter().cloned());
        }

        add_proc_return_vars_before_return(step, ret_var_ops.iter(), &mut BTreeSet::new())?;
    }

    Ok(())
}

/// Pass variables into procedure calls, and read them on return
fn fixup_proc_calls<S>(step: S, visited_steps: &mut BTreeSet<Box<str>>) -> HQResult<()>
where
    S: Deref<Target = RefCell<Step>>,
{
    if visited_steps.contains(step.try_borrow()?.id()) {
        return Ok(());
    }

    visited_steps.insert(step.try_borrow()?.id().into());

    let mut call_indices = vec![];
    for (index, opcode) in step.try_borrow()?.opcodes().iter().enumerate() {
        if matches!(opcode, IrOpcode::procedures_call_warp(_)) {
            call_indices.push(index);
        }

        if let Some(inline_steps) = opcode.inline_steps(true) {
            inline_steps.iter().try_for_each(|inline_step| {
                fixup_proc_calls(Rc::clone(inline_step), visited_steps)
            })?;
        }
    }

    let globally_scoped_variables_num = step.try_borrow()?.globally_scoped_variables_num()?;
    let globally_scoped_variables: Box<[_]> =
        step.try_borrow()?.globally_scoped_variables()?.collect();

    let mut step_mut = step.try_borrow_mut()?;

    step_mut
        .opcodes_mut()
        .reserve_exact(call_indices.len() * globally_scoped_variables_num * 2);
    for call_index in call_indices.iter().rev() {
        #[expect(clippy::range_plus_one, reason = "x+1..=x doesn't make much sense")]
        step_mut.opcodes_mut().splice(
            (call_index + 1)..(call_index + 1),
            globally_scoped_variables.iter().cloned().rev().map(|var| {
                IrOpcode::data_setvariableto(DataSetvariabletoFields {
                    var: RefCell::new(var),
                    local_write: RefCell::new(false),
                    first_write: RefCell::new(false),
                })
            }),
        );
        step_mut.opcodes_mut().splice(
            call_index..call_index,
            globally_scoped_variables.iter().cloned().map(|var| {
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
        let broadcasts = self
            .broadcasts()
            .iter()
            .map(|name| format!(r#""{name}""#))
            .join(", ");
        let steps = self
            .steps()
            .borrow()
            .iter()
            .map(|step| format!("{}", RefCell::borrow(step)))
            .join(", ");
        write!(
            f,
            r#"{{
        "targets": {{{targets}}},
        "global_variables": {{{variables}}},
        "global_lists": {{{lists}}},
        "broadcasts": [{broadcasts}],
        "threads": [{threads}],
        "steps": [{steps}]
    }}"#
        )
    }
}
