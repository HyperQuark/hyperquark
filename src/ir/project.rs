use super::proc::{procs_from_target, ProcMap};
use super::variable::variables_from_target;
use super::{Step, Target, Thread, Variable};
use crate::prelude::*;
use crate::sb3::Sb3Project;

pub type StepSet = IndexSet<Rc<Step>>;

#[derive(Clone, Debug)]
pub struct IrProject {
    threads: RefCell<Box<[Thread]>>,
    steps: RefCell<StepSet>,
    global_variables: BTreeMap<Box<str>, Rc<Variable>>,
    targets: RefCell<BTreeMap<Box<str>, Rc<Target>>>,
}

impl IrProject {
    pub fn threads(&self) -> &RefCell<Box<[Thread]>> {
        &self.threads
    }

    pub fn steps(&self) -> &RefCell<StepSet> {
        &self.steps
    }

    pub fn targets(&self) -> &RefCell<BTreeMap<Box<str>, Rc<Target>>> {
        &self.targets
    }

    pub fn global_variables(&self) -> &BTreeMap<Box<str>, Rc<Variable>> {
        &self.global_variables
    }

    pub fn new(global_variables: BTreeMap<Box<str>, Rc<Variable>>) -> Self {
        IrProject {
            threads: RefCell::new(Box::new([])),
            steps: RefCell::new(Default::default()),
            global_variables,
            targets: RefCell::new(Default::default()),
        }
    }

    pub fn register_step(&self, step: Rc<Step>) -> HQResult<()> {
        self.steps()
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .insert(step);
        Ok(())
    }
}

impl TryFrom<Sb3Project> for Rc<IrProject> {
    type Error = HQError;

    fn try_from(sb3: Sb3Project) -> HQResult<Self> {
        let global_variables = variables_from_target(
            sb3.targets
                .iter()
                .find(|target| target.is_stage)
                .ok_or(make_hq_bad_proj!("missing stage target"))?,
        );

        let project = Rc::new(IrProject::new(global_variables));

        let (threads, targets): (Vec<_>, Vec<_>) = sb3
            .targets
            .iter()
            .map(|target| {
                let variables = variables_from_target(target);
                let procedures = RefCell::new(ProcMap::new());
                let ir_target = Rc::new(Target::new(
                    target.is_stage,
                    variables,
                    Rc::downgrade(&project),
                    procedures,
                ));
                procs_from_target(target, Rc::clone(&ir_target))?;
                let blocks = &target.blocks;
                let threads = blocks
                    .values()
                    .filter_map(|block| {
                        let thread = Thread::try_from_top_block(
                            block,
                            blocks,
                            Rc::downgrade(&ir_target),
                            Rc::downgrade(&project),
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
        let threads = threads.into_iter().flatten().collect::<Box<[_]>>();
        *project
            .threads
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))? = threads;
        *project
            .targets
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))? =
            targets.into_iter().collect();
        Ok(project)
    }
}
