use super::{proc::Proc, IrProject, RcVar};
use crate::{ir::variable::TargetVars, prelude::*};
use core::cell::{Ref, RefMut};

#[derive(Debug, Clone)]
pub struct Target {
    is_stage: bool,
    variables: TargetVars,
    project: Weak<IrProject>,
    procedures: RefCell<BTreeMap<Box<str>, Rc<Proc>>>,
    index: u32,
}

impl Target {
    pub const fn is_stage(&self) -> bool {
        self.is_stage
    }

    pub const fn variables(&self) -> &TargetVars {
        &self.variables
    }

    pub fn project(&self) -> Weak<IrProject> {
        Weak::clone(&self.project)
    }

    pub fn procedures(&self) -> HQResult<Ref<'_, BTreeMap<Box<str>, Rc<Proc>>>> {
        Ok(self.procedures.try_borrow()?)
    }

    pub fn procedures_mut(&self) -> HQResult<RefMut<'_, BTreeMap<Box<str>, Rc<Proc>>>> {
        Ok(self.procedures.try_borrow_mut()?)
    }

    pub const fn index(&self) -> u32 {
        self.index
    }

    pub const fn new(
        is_stage: bool,
        variables: TargetVars,
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

impl fmt::Display for Target {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let is_stage = self.is_stage;
        let index = self.index;
        let variables = self
            .variables
            .iter()
            .map(|(id, var)| format!(r#""{id}": {var}"#))
            .join(", ");
        let procedures = self
            .procedures
            .borrow()
            .iter()
            .map(|(id, proc)| format!(r#""{id}": {proc}"#))
            .join(", ");
        write!(
            f,
            r#"{{
        "is_stage": {is_stage},
        "index": {index},
        "variables": {{ {variables} }},
        "procedures": {{ {procedures} }},
    }}"#
        )
    }
}
