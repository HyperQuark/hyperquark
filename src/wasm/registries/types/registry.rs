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
    pub name: Box<str>,
    pub types: Vec<CompoundType>,
}

impl PartialEq for RecGroup {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl core::hash::Hash for RecGroup {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.name.hash(state);
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

#[must_use]
pub const fn immediate_val_dep(val: &ValType) -> Option<u32> {
    if let ValType::Ref(RefType {
        heap_type: HeapType::Concrete(conc),
        ..
    }) = val
    {
        Some(*conc)
    } else {
        None
    }
}

const fn immediate_field_dep(field: &StorageType) -> Option<u32> {
    match field {
        StorageType::Val(val) => immediate_val_dep(val),
        StorageType::I16 | StorageType::I8 => None,
    }
}

fn immediate_deps(ty: &CompoundType) -> Vec<u32> {
    match ty {
        CompoundType::Array(field, _) => immediate_field_dep(field).into_iter().collect(),
        CompoundType::Struct(fields) => fields
            .iter()
            .map(|field| &field.element_type)
            .filter_map(immediate_field_dep)
            .collect(),
        CompoundType::Function(params, results) => params
            .iter()
            .chain(results)
            .filter_map(immediate_val_dep)
            .collect(),
    }
}

fn extract_rec_group(item: RegistryItem) -> Option<Rc<RecGroup>> {
    match item {
        RegistryItem::RecGroupItem(rec_group, _) => Some(Rc::clone(&rec_group)),
        RegistryItem::Type(_) => None,
    }
}

fn find_compound_type_in_rec_group(
    compound_type: &CompoundType,
    rec_group: &Rc<RecGroup>,
) -> Option<u32> {
    rec_group
        .types
        .iter()
        .find_position(|other| compound_type == *other)
        .map(|(index, _)| index as u32)
}

pub type TypeRegistry = SetRegistry<RegistryItem>;

impl TypeRegistry {
    pub fn register_compound_type<N>(&self, compound_type: CompoundType) -> HQResult<N>
    where
        N: RegistryResult,
    {
        let equivalent_in_rec_group = immediate_deps(&compound_type)
            .into_iter()
            .filter_map(|index| {
                self.registry()
                    .borrow()
                    .get_index(index as usize)
                    .map(|(item, ())| item)
                    .cloned()
            })
            .filter_map(extract_rec_group)
            .find_map(|rec_group| {
                find_compound_type_in_rec_group(&compound_type, &rec_group)
                    .map(|found| (rec_group, found))
            });
        if let Some((rec_group, found_index)) = equivalent_in_rec_group {
            self.register_default(RegistryItem::RecGroupItem(rec_group, found_index))
        } else {
            self.register_default(RegistryItem::Type(compound_type))
        }
    }

    pub fn function<N>(&self, params: Vec<ValType>, returns: Vec<ValType>) -> HQResult<N>
    where
        N: RegistryResult,
    {
        self.register_compound_type(CompoundType::Function(params, returns))
    }

    pub fn array<N>(&self, elem_type: StorageType, mutable: bool) -> HQResult<N>
    where
        N: RegistryResult,
    {
        self.register_compound_type(CompoundType::Array(elem_type, mutable))
    }

    pub fn struct_<N>(&self, fields: Vec<FieldType>) -> HQResult<N>
    where
        N: RegistryResult,
    {
        self.register_compound_type(CompoundType::Struct(fields))
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
                    types
                        .ty()
                        .rec(rec_group.types.iter().cloned().map(Self::type_to_subtype));
                }
                RegistryItem::RecGroupItem(_, _) => (),
            }
        }
    }
}
