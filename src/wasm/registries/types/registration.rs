use core::marker::PhantomData;

use wasm_encoder::{AbstractHeapType, FieldType, HeapType, RefType, StorageType, ValType};

use super::tyfp::List;
use super::{CompoundType, TypeRegistry};
use crate::prelude::*;
use crate::registry::{CompTimeRegistrand, RegistryResult};

pub trait TypeRegistryLike {
    fn register<N>(&self, ty: CompoundType) -> HQResult<N>
    where
        N: RegistryResult;
}

impl TypeRegistryLike for Rc<TypeRegistry> {
    fn register<N>(&self, ty: CompoundType) -> HQResult<N>
    where
        N: RegistryResult,
    {
        self.register_compound_type(ty)
    }
}

// pub trait TypeRegisteringInfo {
//     fn types(&self) -> &impl TypeRegistryLike;
// }

// impl TypeRegisteringInfo for Rc<TypeRegistry> {
//     fn types(&self) -> &impl TypeRegistryLike {
//         self
//     }
// }

pub trait TRecGroupType<T, I: TypeRegistryLike> {
    fn rec_group_ty(registering_info: &I) -> HQResult<T>;
}

impl<T, I> TRecGroupType<u32, I> for T
where
    T: TRecGroupType<CompoundType, I>,
    I: TypeRegistryLike,
{
    default fn rec_group_ty(types: &I) -> HQResult<u32> {
        types.register(T::rec_group_ty(types)?)
    }
}

impl<T, I> TRecGroupType<HeapType, I> for T
where
    T: TRecGroupType<u32, I>,
    I: TypeRegistryLike,
{
    default fn rec_group_ty(types: &I) -> HQResult<HeapType> {
        Ok(HeapType::Concrete(T::rec_group_ty(types)?))
    }
}

pub trait TType<T>: TRecGroupType<T, Rc<TypeRegistry>> {
    fn ty(types: &Rc<TypeRegistry>) -> HQResult<T>;
}

impl<T, U> TType<T> for U
where
    U: TRecGroupType<T, Rc<TypeRegistry>>,
{
    fn ty(types: &Rc<TypeRegistry>) -> HQResult<T> {
        U::rec_group_ty(types)
    }
}

impl<T> CompTimeRegistrand<TypeRegistry, u32> for T
where
    T: TType<u32>,
{
    type Receiver = Rc<TypeRegistry>;

    fn register(types: &Rc<TypeRegistry>) -> HQResult<u32> {
        T::ty(types)
    }
}

pub trait TDefaultable {}

pub trait RegTypeList<I>: List {
    fn register_each(types: &I) -> HQResult<()>;
}

impl<I> RegTypeList<I> for () {
    fn register_each(_types: &I) -> HQResult<()> {
        Ok(())
    }
}

impl<HeadT, Head, Tail, I> RegTypeList<I> for ((HeadT, Head), Tail)
where
    I: TypeRegistryLike,
    Head: TRecGroupType<HeadT, I>,
    Tail: RegTypeList<I>,
{
    fn register_each(types: &I) -> HQResult<()> {
        Head::rec_group_ty(types)?;
        Tail::register_each(types)
    }
}

pub struct TStructRef;
impl<I: TypeRegistryLike> TRecGroupType<CompoundType, I> for TStructRef {
    fn rec_group_ty(_types: &I) -> HQResult<CompoundType> {
        // generic in `T` to make const-evaluation lazy
        // so that the panic is only reached if the function is used
        #[expect(clippy::extra_unused_type_parameters, reason = "explained above")]
        const fn error<T>() {
            const {
                panic!(
                    "tried to register `TStructRef` as a `CompoundType`!
                this implementation exists only to satisfy the trait solver.",
                )
            }
        }
        error::<()>();
        unreachable!();
    }
}

impl<I: TypeRegistryLike> TRecGroupType<HeapType, I> for TStructRef {
    fn rec_group_ty(_types: &I) -> HQResult<HeapType> {
        Ok(HeapType::Abstract {
            shared: false,
            ty: AbstractHeapType::Struct,
        })
    }
}

pub trait TRefType {
    type HeapType;
    const NULLABLE: bool;
}

impl<T, I> TRecGroupType<RefType, I> for T
where
    T: TRefType,
    T::HeapType: TRecGroupType<HeapType, I>,
    I: TypeRegistryLike,
{
    fn rec_group_ty(types: &I) -> HQResult<RefType> {
        Ok(RefType {
            nullable: T::NULLABLE,
            heap_type: T::HeapType::rec_group_ty(types)?,
        })
    }
}

pub struct TNullable<T>(PhantomData<T>);
impl<T> TRefType for TNullable<T> {
    type HeapType = T;
    const NULLABLE: bool = true;
}
impl<T> TDefaultable for TNullable<T> {}

pub struct TNonNullable<T>(PhantomData<T>);
impl<T> TRefType for TNonNullable<T> {
    type HeapType = T;
    const NULLABLE: bool = false;
}

impl<T, I> TRecGroupType<ValType, I> for T
where
    T: TRefType,
    T::HeapType: TRecGroupType<HeapType, I>,
    I: TypeRegistryLike,
{
    fn rec_group_ty(types: &I) -> HQResult<ValType> {
        Ok(ValType::Ref(
            <T as TRecGroupType<RefType, I>>::rec_group_ty(types)?,
        ))
    }
}

pub struct TI32;

impl<I: TypeRegistryLike> TRecGroupType<ValType, I> for TI32 {
    fn rec_group_ty(_types: &I) -> HQResult<ValType> {
        Ok(ValType::I32)
    }
}
impl TDefaultable for TI32 {}

pub trait TFieldType {
    type ValType;
    const MUTABLE: bool;
}

impl<T, I> TRecGroupType<FieldType, I> for T
where
    T: TFieldType,
    I: TypeRegistryLike,
    T::ValType: TRecGroupType<ValType, I>,
{
    fn rec_group_ty(types: &I) -> HQResult<FieldType> {
        Ok(FieldType {
            element_type: StorageType::Val(T::ValType::rec_group_ty(types)?),
            mutable: T::MUTABLE,
        })
    }
}

pub struct TMutField<T>(PhantomData<T>);
pub struct TConstField<T>(PhantomData<T>);

impl<T> TFieldType for TMutField<T> {
    type ValType = T;
    const MUTABLE: bool = true;
}

impl<T> TFieldType for TConstField<T> {
    type ValType = T;
    const MUTABLE: bool = false;
}

impl<T, I: TypeRegistryLike> TRecGroupType<Vec<T>, I> for () {
    fn rec_group_ty(_types: &I) -> HQResult<Vec<T>> {
        Ok(vec![])
    }
}

impl<T, I, Head, Tail> TRecGroupType<Vec<T>, I> for (Head, Tail)
where
    I: TypeRegistryLike,
    Head: TRecGroupType<T, I>,
    Tail: TRecGroupType<Vec<T>, I>,
{
    fn rec_group_ty(types: &I) -> HQResult<Vec<T>> {
        let mut tys = vec![Head::rec_group_ty(types)?];
        tys.extend(Tail::rec_group_ty(types)?);
        Ok(tys)
    }
}

pub struct TStruct<Fields>(PhantomData<Fields>);

impl<Fields, I> TRecGroupType<CompoundType, I> for TStruct<Fields>
where
    I: TypeRegistryLike,
    Fields: TRecGroupType<Vec<FieldType>, I>,
{
    fn rec_group_ty(types: &I) -> HQResult<CompoundType> {
        Ok(CompoundType::Struct(Fields::rec_group_ty(types)?))
    }
}

pub struct TArray<Field>(PhantomData<Field>);

impl<Field, I> TRecGroupType<CompoundType, I> for TArray<Field>
where
    I: TypeRegistryLike,
    Field: TFieldType,
    Field::ValType: TRecGroupType<ValType, I>,
{
    fn rec_group_ty(types: &I) -> HQResult<CompoundType> {
        Ok(CompoundType::Array(
            StorageType::Val(Field::ValType::rec_group_ty(types)?),
            Field::MUTABLE,
        ))
    }
}

pub struct TFunc<Params, Result>(PhantomData<Params>, PhantomData<Result>);

impl<Params, Result, I> TRecGroupType<CompoundType, I> for TFunc<Params, Result>
where
    I: TypeRegistryLike,
    Params: TRecGroupType<Vec<ValType>, I>,
    Result: TRecGroupType<Vec<ValType>, I>,
{
    fn rec_group_ty(types: &I) -> HQResult<CompoundType> {
        Ok(CompoundType::Function(
            Params::rec_group_ty(types)?,
            Result::rec_group_ty(types)?,
        ))
    }
}
