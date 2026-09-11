use wasm_encoder::{
    AbstractHeapType, ArrayType, CompositeInnerType, CompositeType, FieldType, FuncType, HeapType,
    RefType, StorageType, StructType, SubType, TypeSection, ValType,
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

    fn type_to_composite_inner(ty: CompoundType) -> CompositeInnerType {
        match ty {
            CompoundType::Function(params, results) => {
                CompositeInnerType::Func(FuncType::new(params, results))
            }
            CompoundType::Struct(fields) => CompositeInnerType::Struct(StructType {
                fields: fields.into(),
            }),
            CompoundType::Array(element_type, mutable) => {
                CompositeInnerType::Array(ArrayType(FieldType {
                    element_type,
                    mutable,
                }))
            }
        }
    }

    fn type_to_composite(ty: CompoundType) -> CompositeType {
        CompositeType {
            inner: Self::type_to_composite_inner(ty),
            shared: false,
            describes: None,
            descriptor: None,
        }
    }

    fn type_to_subtype(ty: CompoundType) -> SubType {
        SubType {
            is_final: true,
            supertype_idx: None,
            composite_type: Self::type_to_composite(ty),
        }
    }

    pub fn finish(self, types: &mut TypeSection) {
        for ty in self.registry().take().keys().cloned() {
            match ty {
                RegistryItem::Type(ty) => types.ty().subtype(&Self::type_to_subtype(ty)),
                RegistryItem::RecGroupItem(rec_group, 0) => {
                    types.ty().rec(
                        rec_group
                            .types
                            .iter()
                            .map(|(_, compound_ty)| compound_ty.borrow().clone())
                            .map(Self::type_to_subtype),
                    );
                }
                RegistryItem::RecGroupItem(_, _) => (),
            }
        }
    }
}
