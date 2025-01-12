use crate::prelude::*;
use crate::registry::SetRegistry;
use wasm_encoder::{TypeSection, ValType};

pub type TypeRegistry = SetRegistry<(Vec<ValType>, Vec<ValType>)>;

impl TypeRegistry {
    pub fn finish(self, types: &mut TypeSection) {
        for (params, results) in self.registry().take().keys().cloned() {
            types.ty().function(params, results);
        }
    }
}
