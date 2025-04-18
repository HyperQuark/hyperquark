use super::TypeRegistry;
use crate::prelude::*;
use crate::registry::MapRegistry;
use wasm_encoder::{EntityType, ImportSection, ValType};

pub type ExternalFunctionRegistry =
    MapRegistry<(&'static str, Box<str>), (Vec<ValType>, Vec<ValType>)>;

impl ExternalFunctionRegistry {
    pub fn finish(self, imports: &mut ImportSection, type_registry: &TypeRegistry) -> HQResult<()> {
        for ((module, name), (params, results)) in self.registry().take() {
            let type_index = type_registry.register_default((params, results))?;
            imports.import(module, &name, EntityType::Function(type_index));
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum ExternalEnvironment {
    WebBrowser,
}
