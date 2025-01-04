use super::{Step, Target, Thread};
use crate::prelude::*;
use crate::sb3::Sb3Project;

pub type StepSet = IndexSet<Rc<Step>>;

#[derive(Clone, Debug)]
pub struct IrProject {
    threads: RefCell<Box<[Thread]>>,
    steps: RefCell<StepSet>,
    inlined_steps: RefCell<StepSet>,
}

impl IrProject {
    pub fn threads(&self) -> &RefCell<Box<[Thread]>> {
        &self.threads
    }

    pub fn steps(&self) -> &RefCell<StepSet> {
        &self.steps
    }

    pub fn inlined_steps(&self) -> &RefCell<StepSet> {
        &self.inlined_steps
    }

    pub fn new() -> Self {
        IrProject {
            threads: RefCell::new(Box::new([])),
            steps: RefCell::new(Default::default()),
            inlined_steps: RefCell::new(Default::default()),
        }
    }

    pub fn register_step(&self, step: Rc<Step>) {
        self.steps().borrow_mut().insert(step);
    }
}

impl TryFrom<Sb3Project> for Rc<IrProject> {
    type Error = HQError;

    fn try_from(sb3: Sb3Project) -> HQResult<Self> {
        let project = Rc::new(IrProject::new());

        let threads = sb3
            .targets
            .iter()
            .map(|target| {
                let ir_target = Rc::new(Target::new(target.name.clone(), target.is_stage));
                let blocks = &target.blocks;
                blocks
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
                    .collect::<HQResult<Box<[_]>>>()
            })
            .collect::<HQResult<Box<[_]>>>()?
            .iter()
            .flatten()
            .cloned()
            .collect::<Box<[_]>>();
        *project.threads.borrow_mut() = threads;
        Ok(project)
    }
}
