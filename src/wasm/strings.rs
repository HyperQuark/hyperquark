use crate::prelude::*;
use crate::registry::SetRegistry;
use wasm_encoder::{EntityType, GlobalType, ImportSection, ValType};

pub type StringRegistry = SetRegistry<Box<str>>;

impl StringRegistry {
    pub fn finish(self, imports: &mut ImportSection) {
        for string in self.registry().take().keys() {
            imports.import(
                "",
                string,
                EntityType::Global(GlobalType {
                    val_type: ValType::EXTERNREF,
                    mutable: false,
                    shared: false,
                }),
            );
        }
    }
}
