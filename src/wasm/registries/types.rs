use crate::prelude::*;
use crate::registry::SetRegistry;
use wasm_encoder::{FieldType, StorageType, TypeSection, ValType};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WasmType {
    Function(Vec<ValType>, Vec<ValType>),
    Array(StorageType, bool),
    Struct(Vec<FieldType>),
}

pub type TypeRegistry = SetRegistry<WasmType>;

impl TypeRegistry {
    pub fn function<N>(&self, params: Vec<ValType>, returns: Vec<ValType>) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.register_default(WasmType::Function(params, returns))
    }

    pub fn array<N>(&self, elem_type: StorageType, mutable: bool) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.register_default(WasmType::Array(elem_type, mutable))
    }

    pub fn struct_<N>(&self, fields: Vec<FieldType>) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.register_default(WasmType::Struct(fields))
    }

    pub fn finish(self, types: &mut TypeSection) {
        for ty in self.registry().take().keys().cloned() {
            match ty {
                WasmType::Function(params, results) => types.ty().function(params, results),
                WasmType::Array(elem_type, mutable) => types.ty().array(&elem_type, mutable),
                WasmType::Struct(fields) => types.ty().struct_(fields),
            }
        }
    }
}
