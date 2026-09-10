use core::marker::PhantomData;

use crate::wasm::registries::types::{Bool, Func, List, Map, TFieldType, TFunc, TStruct};

pub trait FuncSubTypes {
    type Params: List;
    type Results: List;
}

pub struct IsFieldType;
impl Func for IsFieldType {
    type Func<T> = FieldTypeTester<T>;
}

pub struct FieldTypeTester<T>(PhantomData<T>);

impl<T> Bool for FieldTypeTester<T> {
    default const BOOL: bool = false;
}

impl<HeadT, Head> Bool for FieldTypeTester<(HeadT, Head)>
where
    Head: TFieldType,
{
    const BOOL: bool = true;
}

pub struct ExtractValFromField;

pub struct FieldValExtractor<T>(PhantomData<T>);

pub trait HasValType {
    type ValType;
}

impl<T> HasValType for FieldValExtractor<T> {
    default type ValType = !;
}

impl<T> HasValType for FieldValExtractor<T>
where
    T: TFieldType,
{
    type ValType = T::ValType;
}

impl Func for ExtractValFromField {
    type Func<T> = <FieldValExtractor<T> as HasValType>::ValType;
}

impl<Params, Results> FuncSubTypes for TFunc<Params, Results>
where
    Params: List,
    Results: List,
{
    type Params = Params;
    type Results = Results;
}

pub trait StructSubTypes {
    type Fields: List;
}

impl<Fields> StructSubTypes for TStruct<Fields>
where
    Fields: List + Map<ExtractValFromField>,
{
    type Fields = Fields::Mapped;
}

pub trait ArraySubType {
    type Field;
}
