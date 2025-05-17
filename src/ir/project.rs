use super::proc::{procs_from_target, ProcMap};
use super::variable::variables_from_target;
use super::{RcVar, Step, Target, Thread};
use crate::prelude::*;
use crate::sb3::Sb3Project;

pub type StepSet = IndexSet<Rc<Step>>;

#[derive(Clone, Debug)]
pub struct IrProject {
    threads: RefCell<Box<[Thread]>>,
    steps: RefCell<StepSet>,
    global_variables: BTreeMap<Box<str>, RcVar>,
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

    pub const fn global_variables(&self) -> &BTreeMap<Box<str>, RcVar> {
        &self.global_variables
    }

    pub fn new(global_variables: BTreeMap<Box<str>, RcVar>) -> Self {
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
}

impl TryFrom<Sb3Project> for Rc<IrProject> {
    type Error = HQError;

    fn try_from(sb3: Sb3Project) -> HQResult<Self> {
        let global_variables = variables_from_target(
            sb3.targets
                .iter()
                .find(|target| target.is_stage)
                .ok_or_else(|| make_hq_bad_proj!("missing stage target"))?,
        );

        let project = Self::new(IrProject::new(global_variables));

        let (threads_vec, targets): (Vec<_>, Vec<_>) = sb3
            .targets
            .iter()
            .enumerate()
            .map(|(index, target)| {
                let variables = variables_from_target(target);
                let procedures = RefCell::new(ProcMap::new());
                let ir_target = Rc::new(Target::new(
                    target.is_stage,
                    variables,
                    Self::downgrade(&project),
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
                            &Self::downgrade(&project),
                            target.comments.clone().iter().any(|(_id, comment)| {
                                matches!(comment.block_id.clone(), Some(d) if &d == id)
                                    && *comment.text.clone() == *"hq-dbg"
                            }),
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
        Ok(project)
    }
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
