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
        let globals = Rc::new(GlobalRegistry::default());
        let variables = VariableRegistry::new(&globals);
        Self {
            globals,
            variables,
            strings: StringRegistry::default(),
            external_functions: ExternalFunctionRegistry::default(),
            tables: TableRegistry::default(),
            types: TypeRegistry::default(),
        }
    }
}

impl Registries {
    pub const fn strings(&self) -> &StringRegistry {
        &self.strings
    }

    pub const fn external_functions(&self) -> &ExternalFunctionRegistry {
        &self.external_functions
    }

    pub const fn types(&self) -> &TypeRegistry {
        &self.types
    }

    pub const fn tables(&self) -> &TableRegistry {
        &self.tables
    }

    pub fn globals(&self) -> &GlobalRegistry {
        &self.globals
    }

    pub const fn variables(&self) -> &VariableRegistry {
        &self.variables
    }
}
