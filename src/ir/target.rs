use super::Variable;
use crate::prelude::*;

#[derive(Debug, Clone, PartialEq)]
pub struct Target {
    is_stage: bool,
    variables: BTreeMap<Box<str>, Rc<Variable>>,
}

impl Target {
    pub fn is_stage(&self) -> bool {
        self.is_stage
    }

    pub fn variables(&self) -> &BTreeMap<Box<str>, Rc<Variable>> {
        &self.variables
    }

    pub fn new(is_stage: bool, variables: BTreeMap<Box<str>, Rc<Variable>>) -> Self {
        Target {
            is_stage,
            variables,
        }
    }
}
