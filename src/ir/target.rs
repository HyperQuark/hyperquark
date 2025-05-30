use super::{proc::Proc, IrProject, Variable};
use crate::prelude::*;
use core::cell::{Ref, RefMut};

#[derive(Debug, Clone)]
pub struct Target {
    is_stage: bool,
    variables: BTreeMap<Box<str>, Rc<Variable>>,
    project: Weak<IrProject>,
    procedures: RefCell<BTreeMap<Box<str>, Rc<Proc>>>,
    index: u32,
}

impl Target {
    pub const fn is_stage(&self) -> bool {
        self.is_stage
    }

    pub const fn variables(&self) -> &BTreeMap<Box<str>, Rc<Variable>> {
        &self.variables
    }

    pub fn project(&self) -> Weak<IrProject> {
        Weak::clone(&self.project)
    }

    pub fn procedures(&self) -> HQResult<Ref<BTreeMap<Box<str>, Rc<Proc>>>> {
        Ok(self.procedures.try_borrow()?)
    }

    pub fn procedures_mut(&self) -> HQResult<RefMut<BTreeMap<Box<str>, Rc<Proc>>>> {
        Ok(self.procedures.try_borrow_mut()?)
    }

    pub const fn index(&self) -> u32 {
        self.index
    }

    pub const fn new(
        is_stage: bool,
        variables: BTreeMap<Box<str>, Rc<Variable>>,
        project: Weak<IrProject>,
        procedures: RefCell<BTreeMap<Box<str>, Rc<Proc>>>,
        index: u32,
    ) -> Self {
        Self {
            is_stage,
            variables,
            project,
            procedures,
            index,
        }
    }
}
