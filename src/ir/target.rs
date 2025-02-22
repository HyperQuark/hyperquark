use super::{IrProject, Variable};
use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct Target {
    is_stage: bool,
    variables: BTreeMap<Box<str>, Rc<Variable>>,
    project: Weak<IrProject>,
}

impl Target {
    pub fn is_stage(&self) -> bool {
        self.is_stage
    }

    pub fn variables(&self) -> &BTreeMap<Box<str>, Rc<Variable>> {
        &self.variables
    }

    pub fn project(&self) -> Weak<IrProject> {
        Weak::clone(&self.project)
    }

    pub fn new(
        is_stage: bool,
        variables: BTreeMap<Box<str>, Rc<Variable>>,
        project: Weak<IrProject>,
    ) -> Self {
        Target {
            is_stage,
            variables,
            project,
        }
    }
}
