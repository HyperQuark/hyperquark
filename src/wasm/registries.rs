use super::{ExternalFunctionRegistry, StringRegistry, TableRegistry, TypeRegistry};

#[derive(Default)]
pub struct Registries {
    strings: StringRegistry,
    external_functions: ExternalFunctionRegistry,
    types: TypeRegistry,
    tables: TableRegistry,
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
}
