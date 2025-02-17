use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    name: Box<str>,
    is_stage: bool,
}

impl Target {
    pub fn name(&self) -> &Box<str> {
        &self.name
    }

    pub fn is_stage(&self) -> bool {
        self.is_stage
    }

    pub fn new(name: Box<str>, is_stage: bool) -> Self {
        Target { name, is_stage }
    }
}
