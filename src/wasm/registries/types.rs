use wasm_encoder::{
    AbstractHeapType, FieldType, HeapType, RefType, StorageType, TypeSection, ValType,
};

use core::marker::PhantomData;

use crate::ir::RcVar;
use crate::prelude::*;
use crate::registry::{CompTimeRegistrand, RegistryResult, SetRegistry};
use crate::wasm::WasmProject;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum CompoundType {
    Function(Vec<ValType>, Vec<ValType>),
    Array(StorageType, bool),
    Struct(Vec<FieldType>),
}

pub type TypeRegistry = SetRegistry<CompoundType>;

impl TypeRegistry {
    pub fn function<N>(&self, params: Vec<ValType>, returns: Vec<ValType>) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.register_default(CompoundType::Function(params, returns))
    }

    pub fn array<N>(&self, elem_type: StorageType, mutable: bool) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.register_default(CompoundType::Array(elem_type, mutable))
    }

    pub fn struct_<N>(&self, fields: Vec<FieldType>) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.register_default(CompoundType::Struct(fields))
    }

    pub const STRUCT_REF: ValType = ValType::Ref(RefType {
        nullable: true,
        heap_type: HeapType::Abstract {
            shared: false,
            ty: AbstractHeapType::Struct,
        },
    });

    pub fn step_func(&self) -> HQResult<u32> {
        self.function(vec![ValType::I32, Self::STRUCT_REF], vec![])
    }

    pub fn dyn_array_container(&self, field: ValType) -> HQResult<u32> {
        let arr_type = self.array(StorageType::Val(field), true)?;
        self.struct_(vec![FieldType {
            element_type: Self::ref_storage(arr_type, false),
            mutable: true,
        }])
    }

    pub fn ref_(heap_type: u32, nullable: bool) -> RefType {
        RefType {
            nullable,
            heap_type: HeapType::Concrete(heap_type),
        }
    }

    pub fn ref_val(heap_type: u32, nullable: bool) -> ValType {
        ValType::Ref(Self::ref_(heap_type, nullable))
    }

    pub fn ref_storage(heap_type: u32, nullable: bool) -> StorageType {
        StorageType::Val(Self::ref_val(heap_type, nullable))
    }

    pub fn stack_struct_type(&self) -> HQResult<u32> {
        self.struct_(vec![
            FieldType {
                element_type: Self::ref_storage(self.step_func()?, false),
                mutable: true,
            },
            FieldType {
                element_type: StorageType::Val(Self::STRUCT_REF),
                mutable: false,
            },
        ])
    }

    pub fn stack_array_type(&self) -> HQResult<u32> {
        self.array(Self::ref_storage(self.stack_struct_type()?, true), true)
    }

    pub fn thread_struct_type(&self) -> HQResult<u32> {
        self.struct_(vec![
            FieldType {
                element_type: StorageType::Val(ValType::I32),
                mutable: true,
            },
            FieldType {
                element_type: Self::ref_storage(self.stack_array_type()?, false),
                mutable: true,
            },
        ])
    }

    pub fn thread_array_type(&self) -> HQResult<u32> {
        self.array(Self::ref_storage(self.thread_struct_type()?, true), true)
    }

    pub fn thread_list_struct_type(&self) -> HQResult<u32> {
        self.struct_(vec![
            // target index
            FieldType {
                element_type: StorageType::Val(ValType::I32),
                mutable: true,
            },
            // number of threads
            FieldType {
                element_type: StorageType::Val(ValType::I32),
                mutable: true,
            },
            FieldType {
                element_type: Self::ref_storage(self.thread_array_type()?, false),
                mutable: true,
            },
        ])
    }

    pub fn thread_list_array_type(&self) -> HQResult<u32> {
        self.array(
            StorageType::Val(ValType::Ref(RefType {
                nullable: true,
                heap_type: HeapType::Concrete(self.thread_list_struct_type()?),
            })),
            true,
        )
    }

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

    pub fn finish(self, types: &mut TypeSection) {
        for ty in self.registry().take().keys().cloned() {
            match ty {
                CompoundType::Function(params, results) => types.ty().function(params, results),
                CompoundType::Array(elem_type, mutable) => types.ty().array(&elem_type, mutable),
                CompoundType::Struct(fields) => types.ty().struct_(fields),
            }
        }
    }
}

pub trait THeapType: CompTimeRegistrand<TypeRegistry, u32> {}
impl<T> THeapType for T where T: CompTimeRegistrand<TypeRegistry, u32> {}

pub trait TRefType {
    type HeapType: THeapType;
    const NULLABLE: bool;
}

pub struct TNullable<T>(PhantomData<T>);

pub struct TNonNullable<T>(PhantomData<T>);

impl<T> TRefType for TNullable<T> where T: THeapType {
    type HeapType = T;
    const NULLABLE: bool = true;
}

impl<T> TRefType for TNonNullable<T> where T: THeapType {
    type HeapType = T;
    const NULLABLE: bool = true;
}

pub trait TValType {
    fn val_type(types: &TypeRegistry) -> ValType;
}

pub struct DynArray<T>(PhantomData<T>);

impl<T> CompTimeRegistrand<TypeRegistry, u32> for DynArray<T> where T: TRefType {
    fn register(types: &TypeRegistry) -> HQResult<u32> {
        types.dyn_array_container(
            TypeRegistry::ref_val(
                T::HeapType::register(types)?,
                T::NULLABLE,
            )
        )
    }
}

pub trait TFieldType {
    type ValType: TValType;
    const MUTABLE: bool;
}

trait CompTypeList {
    fn fields(types: &TypeRegistry) -> Vec<FieldType>;
}

impl CompTypeList for () {
    fn fields(_: &TypeRegistry) -> Vec<FieldType> {
        vec![]
    }
}

impl<Head, Tail> CompTypeList for (Head, Tail) where Head: CompTypeList, Tail: TFieldType {
    fn fields(types: &TypeRegistry) -> Vec<FieldType> {
        let mut fields = Head::fields(types);
        fields.push(
            FieldType {
                mutable: Tail::MUTABLE,
                element_type: StorageType::Val(Tail::ValType::val_type(types)),
            }
        );
        fields
    }
}

struct TStruct<Fields>(PhantomData<Fields>);

impl<Fields> CompTimeRegistrand<TypeRegistry, u32> for TStruct<Fields> where Fields: CompTypeList {
    fn register(types: &TypeRegistry) -> HQResult<u32> {
        types.struct_(
            Fields::fields(types)
        )
    }
}
