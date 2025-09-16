pub mod functions;
pub mod globals;
pub mod strings;
pub mod tables;
pub mod targets;
pub mod types;
pub mod variables;

use crate::prelude::*;
pub use functions::ExternalFunctionRegistry;
pub use globals::{GlobalExportable, GlobalMutable, GlobalRegistry};
pub use strings::StringRegistry;
pub use tables::{StepsTable, StringsTable, TableRegistry, ThreadsTable};
pub use targets::SpriteRegistry;
pub use types::TypeRegistry;
pub use variables::VariableRegistry;

pub struct Registries {
    strings: StringRegistry,
    external_functions: ExternalFunctionRegistry,
    types: TypeRegistry,
    tables: TableRegistry,
    globals: Rc<GlobalRegistry>,
    variables: VariableRegistry,
    sprites: SpriteRegistry,
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
            sprites: SpriteRegistry::default(),
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

    pub const fn sprites(&self) -> &SpriteRegistry {
        &self.sprites
    }
}
