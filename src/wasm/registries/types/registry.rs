use wasm_encoder::{
    AbstractHeapType, FieldType, HeapType, RefType, StorageType, TypeSection, ValType,
};

use crate::ir::RcVar;
use crate::prelude::*;
use crate::registry::{RegistryResult, SetRegistry};
use crate::wasm::WasmProject;

#[derive(Clone, Debug, Eq)]
pub struct RecGroup {
    pub types: Vec<(&'static str, RefCell<CompoundType>)>,
}

impl PartialEq for RecGroup {
    fn eq(&self, other: &Self) -> bool {
        self.types.len() == other.types.len()
            && self
                .types
                .iter()
                .zip(&other.types)
                .all(|((fst, _), (snd, _))| fst == snd)
    }
}

impl core::hash::Hash for RecGroup {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.types
            .iter()
            .map(|(tystr, _)| tystr)
            .collect::<Box<[_]>>()
            .hash(state);
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CompoundType {
    Function(Vec<ValType>, Vec<ValType>),
    Array(StorageType, bool),
    Struct(Vec<FieldType>),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RegistryItem {
    Type(CompoundType),
    RecGroupItem(Rc<RecGroup>, u32),
}

pub type TypeRegistry = SetRegistry<RegistryItem>;

impl TypeRegistry {
    pub fn function<N>(&self, params: Vec<ValType>, returns: Vec<ValType>) -> HQResult<N>
    where
        N: RegistryResult,
    {
        self.register_default(RegistryItem::Type(CompoundType::Function(params, returns)))
    }

    pub fn array<N>(&self, elem_type: StorageType, mutable: bool) -> HQResult<N>
    where
        N: RegistryResult,
    {
        self.register_default(RegistryItem::Type(CompoundType::Array(elem_type, mutable)))
    }

    pub fn struct_<N>(&self, fields: Vec<FieldType>) -> HQResult<N>
    where
        N: RegistryResult,
    {
        self.register_default(RegistryItem::Type(CompoundType::Struct(fields)))
    }

    pub const STRUCT_REF: ValType = ValType::Ref(RefType {
        nullable: true,
        heap_type: HeapType::Abstract {
            shared: false,
            ty: AbstractHeapType::Struct,
        },
    });

    pub fn proc_arg_struct_type(
        &self,
        arg_vars: &core::cell::Ref<'_, Vec<RcVar>>,
    ) -> HQResult<u32> {
        self.struct_(
            arg_vars
                .iter()
                .map(|var| {
                    Ok(FieldType {
                        mutable: false,
                        element_type: StorageType::Val(WasmProject::ir_type_to_wasm(
                            *var.possible_types(),
                        )),
                    })
                })
                .collect::<HQResult<Vec<_>>>()?,
        )
    }

    fn finish_type(ty: CompoundType, types: &mut TypeSection) {
        match ty {
            CompoundType::Function(params, results) => types.ty().function(params, results),
            CompoundType::Array(elem_type, mutable) => types.ty().array(&elem_type, mutable),
            CompoundType::Struct(fields) => types.ty().struct_(fields),
        }
    }

    pub fn finish(self, types: &mut TypeSection) {
        for ty in self.registry().take().keys().cloned() {
            match ty {
                RegistryItem::Type(ty) => Self::finish_type(ty, types),
                RegistryItem::RecGroupItem(rec_group, index) => {
                    Self::finish_type(rec_group.types[index as usize].1.borrow().clone(), types);
                }
            }
        }
    }
}