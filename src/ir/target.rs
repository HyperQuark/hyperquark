use super::{IrProject, proc::Proc};
use crate::ir::variable::TargetVars;
use crate::prelude::*;
use crate::sb3::CostumeDataFormat;
use core::cell::{Ref, RefMut};

#[derive(Debug, Clone, PartialEq)]
pub struct IrCostume {
    pub name: Box<str>,
    pub data_format: CostumeDataFormat,
    pub md5ext: Box<str>,
}

#[derive(Debug, Clone)]
pub struct Target {
    is_stage: bool,
    variables: TargetVars,
    project: Weak<IrProject>,
    procedures: RefCell<BTreeMap<Box<str>, Rc<Proc>>>,
    index: u32,
    costumes: Box<[IrCostume]>,
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

    pub const fn costumes(&self) -> &Box<[IrCostume]> {
        &self.costumes
    }

    pub const fn new(
        is_stage: bool,
        variables: TargetVars,
        project: Weak<IrProject>,
        procedures: RefCell<BTreeMap<Box<str>, Rc<Proc>>>,
        index: u32,
        costumes: Box<[IrCostume]>,
    ) -> Self {
        Self {
            is_stage,
            variables,
            project,
            procedures,
            index,
            costumes,
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

impl core::hash::Hash for Target {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.index().hash(state);
    }
}

impl core::cmp::PartialEq for Target {
    fn eq(&self, other: &Self) -> bool {
        self.index() == other.index()
    }
}

impl core::cmp::Eq for Target {}
