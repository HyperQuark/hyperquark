use super::{
    ExternalFunctionRegistry, GlobalRegistry, StringRegistry, TableRegistry, TypeRegistry,
    VariableRegistry,
};
use crate::prelude::*;

pub struct Registries {
    strings: StringRegistry,
    external_functions: ExternalFunctionRegistry,
    types: TypeRegistry,
    tables: TableRegistry,
    globals: Rc<GlobalRegistry>,
    variables: VariableRegistry,
}

impl Default for Registries {
    fn default() -> Self {
        let globals = Default::default();
        let variables = VariableRegistry::new(&globals);
        Registries {
            globals,
            variables,
            strings: Default::default(),
            external_functions: Default::default(),
            tables: Default::default(),
            types: Default::default(),
        }
    }
}

impl Registries {
    pub fn strings(&self) -> &StringRegistry {
        &self.strings
    }

    pub fn external_functions(&self) -> &ExternalFunctionRegistry {
        &self.external_functions
    }

    pub fn types(&self) -> &TypeRegistry {
        &self.types
    }

    pub fn tables(&self) -> &TableRegistry {
        &self.tables
    }

    pub fn globals(&self) -> &GlobalRegistry {
        &self.globals
    }

    pub fn variables(&self) -> &VariableRegistry {
        &self.variables
    }
}
