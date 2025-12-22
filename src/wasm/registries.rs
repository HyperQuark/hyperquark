pub mod functions;
pub mod globals;
pub mod lists;
pub mod strings;
pub mod tables;
pub mod targets;
pub mod types;
pub mod variables;

use crate::prelude::*;
pub use functions::{ExternalFunctionRegistry, StaticFunctionRegistry};
pub use globals::{GlobalExportable, GlobalMutable, GlobalRegistry};
pub use lists::ListRegistry;
pub use strings::{StringRegistry, TabledStringRegistry};
pub use tables::{StepsTable, StringsTable, TableRegistry, ThreadsTable};
pub use targets::SpriteRegistry;
pub use types::TypeRegistry;
pub use variables::VariableRegistry;

pub struct Registries {
    strings: Rc<StringRegistry>,
    tabled_strings: Rc<TabledStringRegistry>,
    external_functions: ExternalFunctionRegistry,
    static_functions: StaticFunctionRegistry,
    types: Rc<TypeRegistry>,
    tables: TableRegistry,
    globals: Rc<GlobalRegistry>,
    variables: VariableRegistry,
    sprites: SpriteRegistry,
    lists: ListRegistry,
}

impl Default for Registries {
    fn default() -> Self {
        let globals = Rc::new(GlobalRegistry::default());
        let strings = Rc::new(StringRegistry::default());
        let tabled_strings = Rc::new(TabledStringRegistry::default());
        let types = Rc::new(TypeRegistry::default());
        let variables = VariableRegistry::new(&globals, &strings, &tabled_strings);
        let lists = ListRegistry::new(&globals, &types, &strings, &tabled_strings);
        Self {
            globals,
            variables,
            strings,
            tabled_strings,
            external_functions: ExternalFunctionRegistry::default(),
            tables: TableRegistry::default(),
            types,
            sprites: SpriteRegistry::default(),
            static_functions: StaticFunctionRegistry::default(),
            lists,
        }
    }
}

impl Registries {
    pub const fn strings(&self) -> &Rc<StringRegistry> {
        &self.strings
    }

    pub const fn tabled_strings(&self) -> &Rc<TabledStringRegistry> {
        &self.tabled_strings
    }

    pub const fn external_functions(&self) -> &ExternalFunctionRegistry {
        &self.external_functions
    }

    pub const fn static_functions(&self) -> &StaticFunctionRegistry {
        &self.static_functions
    }

    pub const fn types(&self) -> &Rc<TypeRegistry> {
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

    pub const fn lists(&self) -> &ListRegistry {
        &self.lists
    }
}
