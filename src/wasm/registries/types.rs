use core::marker::PhantomData;

use wasm_encoder::{
    AbstractHeapType, FieldType, HeapType, RefType, StorageType, TypeSection, ValType,
};

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
        N: RegistryResult,
    {
        self.register_default(CompoundType::Function(params, returns))
    }

    pub fn array<N>(&self, elem_type: StorageType, mutable: bool) -> HQResult<N>
    where
        N: RegistryResult,
    {
        self.register_default(CompoundType::Array(elem_type, mutable))
    }

    pub fn struct_<N>(&self, fields: Vec<FieldType>) -> HQResult<N>
    where
        N: RegistryResult,
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

pub trait THeapType {
    fn heap_type(types: &TypeRegistry) -> HQResult<HeapType>;
}

impl<T> THeapType for T
where
    T: CompTimeRegistrand<TypeRegistry, u32>,
{
    fn heap_type(types: &TypeRegistry) -> HQResult<HeapType> {
        Ok(HeapType::Concrete(types.register_comp::<T, u32>()?))
    }
}

pub struct TStructRef;
impl THeapType for TStructRef {
    fn heap_type(_types: &TypeRegistry) -> HQResult<HeapType> {
        Ok(HeapType::Abstract {
            shared: false,
            ty: AbstractHeapType::Struct,
        })
    }
}

pub trait TRefType {
    type HeapType: THeapType;
    const NULLABLE: bool;

    fn ref_type(types: &TypeRegistry) -> HQResult<RefType> {
        Ok(RefType {
            nullable: Self::NULLABLE,
            heap_type: Self::HeapType::heap_type(types)?,
        })
    }
}

pub struct TNullable<T>(PhantomData<T>);
impl<T> TRefType for TNullable<T>
where
    T: THeapType,
{
    type HeapType = T;
    const NULLABLE: bool = true;
}
impl<T: THeapType> TDefaultable for TNullable<T> {}

pub struct TNonNullable<T>(PhantomData<T>);
impl<T> TRefType for TNonNullable<T>
where
    T: THeapType,
{
    type HeapType = T;
    const NULLABLE: bool = true;
}

pub trait TValType {
    fn val_type(types: &TypeRegistry) -> HQResult<ValType>;
}

impl<T> TValType for T
where
    T: TRefType,
{
    fn val_type(types: &TypeRegistry) -> HQResult<ValType> {
        Ok(ValType::Ref(T::ref_type(types)?))
    }
}

pub struct TI32;

impl TValType for TI32 {
    fn val_type(_types: &TypeRegistry) -> HQResult<ValType> {
        Ok(ValType::I32)
    }
}
impl TDefaultable for TI32 {}

pub trait TFieldType {
    type ValType: TValType;
    const MUTABLE: bool;

    fn field_type(types: &TypeRegistry) -> HQResult<FieldType> {
        Ok(FieldType {
            element_type: StorageType::Val(Self::ValType::val_type(types)?),
            mutable: Self::MUTABLE,
        })
    }
}

pub struct TMutField<T>(PhantomData<T>);
pub struct TConstField<T>(PhantomData<T>);

impl<T: TValType> TFieldType for TMutField<T> {
    type ValType = T;
    const MUTABLE: bool = true;
}

impl<T: TValType> TFieldType for TConstField<T> {
    type ValType = T;
    const MUTABLE: bool = false;
}

pub trait TDefaultable: TValType {}

trait TFieldList {
    fn fields(types: &TypeRegistry) -> HQResult<Vec<FieldType>>;
}

impl TFieldList for () {
    fn fields(_: &TypeRegistry) -> HQResult<Vec<FieldType>> {
        Ok(vec![])
    }
}

impl<Head, Tail> TFieldList for (Head, Tail)
where
    Head: TFieldList,
    Tail: TFieldType,
{
    fn fields(types: &TypeRegistry) -> HQResult<Vec<FieldType>> {
        let mut fields = Head::fields(types)?;
        fields.push(Tail::field_type(types)?);
        Ok(fields)
    }
}

pub struct TStruct<Fields>(PhantomData<Fields>);

impl<Fields> CompTimeRegistrand<TypeRegistry, u32> for TStruct<Fields>
where
    Fields: TFieldList,
{
    fn register(types: &TypeRegistry) -> HQResult<u32> {
        types.struct_(Fields::fields(types)?)
    }
}

pub struct TArray<Field>(PhantomData<Field>);

impl<Field: TFieldType> CompTimeRegistrand<TypeRegistry, u32> for TArray<Field> {
    fn register(types: &TypeRegistry) -> HQResult<u32> {
        types.array(
            StorageType::Val(Field::ValType::val_type(types)?),
            Field::MUTABLE,
        )
    }
}

trait TValTypeList {
    fn val_types(types: &TypeRegistry) -> HQResult<Vec<ValType>>;
}

impl TValTypeList for () {
    fn val_types(_: &TypeRegistry) -> HQResult<Vec<ValType>> {
        Ok(vec![])
    }
}

impl<Head, Tail> TValTypeList for (Head, Tail)
where
    Head: TValTypeList,
    Tail: TValType,
{
    fn val_types(types: &TypeRegistry) -> HQResult<Vec<ValType>> {
        let mut val_types = Head::val_types(types)?;
        val_types.push(Tail::val_type(types)?);
        Ok(val_types)
    }
}

pub struct TFunc<Params, Result>(PhantomData<Params>, PhantomData<Result>);

impl<Params, Result> CompTimeRegistrand<TypeRegistry, u32> for TFunc<Params, Result>
where
    Params: TValTypeList,
    Result: TValTypeList,
{
    fn register(types: &TypeRegistry) -> HQResult<u32> {
        types.function(Params::val_types(types)?, Result::val_types(types)?)
    }
}

pub type TStepFunc = TFunc<(((), TI32), TNullable<TStructRef>), ()>;

pub type TDynArrayField<T> = TArray<TMutField<T>>;
pub type TDynArray<T> = TStruct<(
    ((), TMutField<TNonNullable<TDynArrayField<T>>>),
    TMutField<TI32>,
)>;

pub type TStackStruct = TStruct<(
    ((), TMutField<TNonNullable<TStepFunc>>),
    TConstField<TNullable<TStructRef>>,
)>;

pub type TStackArray = TDynArray<TNullable<TStackStruct>>;

pub type TThreadArray = TDynArray<TNullable<TStackArray>>;

pub type TTargetThreadsStruct = TStruct<(
    (((), TMutField<TI32>), TMutField<TI32>),
    TMutField<TNonNullable<TThreadArray>>,
)>;

pub type TTargetThreadArray = TArray<TMutField<TNonNullable<TTargetThreadsStruct>>>;
