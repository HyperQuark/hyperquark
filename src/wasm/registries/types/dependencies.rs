use core::marker::PhantomData;

use wasm_encoder::{FieldType, HeapType, RefType, ValType};

use super::TypeRegistry;
use super::rec_group::RecGroupRegistry;
use super::registration::{
    TArray, TFieldType, TFunc, TI32, TRecGroupType, TRefType, TStruct, TStructRef, TType,
};
use super::tyfp::List;
use crate::prelude::*;
use crate::wasm::registries::types::TRecGroupItem;

pub trait HasTypeDependencies<T> {
    type Dependencies: List;
    type RecGroupDependencies: List;
}

impl HasTypeDependencies<HeapType> for TStructRef {
    type Dependencies = ();
    type RecGroupDependencies = ();
}

impl<T> HasTypeDependencies<RefType> for T
where
    T: TRefType,
    T::HeapType: HasTypeDependencies<HeapType>,
{
    type Dependencies = <T::HeapType as HasTypeDependencies<HeapType>>::Dependencies;
    type RecGroupDependencies =
        <T::HeapType as HasTypeDependencies<HeapType>>::RecGroupDependencies;
}

impl<T> HasTypeDependencies<ValType> for T
where
    T: TRefType,
    T::HeapType: HasTypeDependencies<HeapType>,
{
    type Dependencies = <T::HeapType as HasTypeDependencies<HeapType>>::Dependencies;
    type RecGroupDependencies =
        <T::HeapType as HasTypeDependencies<HeapType>>::RecGroupDependencies;
}

impl HasTypeDependencies<ValType> for TI32 {
    type Dependencies = ();
    type RecGroupDependencies = ();
}

impl<T> HasTypeDependencies<FieldType> for T
where
    T: TFieldType,
    T::ValType: HasTypeDependencies<ValType>,
{
    type Dependencies = <T::ValType as HasTypeDependencies<ValType>>::Dependencies;
    type RecGroupDependencies = <T::ValType as HasTypeDependencies<ValType>>::RecGroupDependencies;
}

pub struct TTypeListMarker<T>(PhantomData<T>);

impl<T> HasTypeDependencies<TTypeListMarker<T>> for () {
    type Dependencies = ();
    type RecGroupDependencies = ();
}

impl<T, Head, Tail> HasTypeDependencies<TTypeListMarker<T>> for (Head, Tail)
where
    Head: HasTypeDependencies<T>,
    Head::Dependencies: List,
    Head::RecGroupDependencies: List,
    Tail: HasTypeDependencies<TTypeListMarker<T>>,
    Tail::Dependencies: List,
    Tail::RecGroupDependencies: List,
{
    type Dependencies = <<Head as HasTypeDependencies<T>>::Dependencies as List>::Concat<
        <Tail as HasTypeDependencies<TTypeListMarker<T>>>::Dependencies,
    >;
    type RecGroupDependencies =
        <<Head as HasTypeDependencies<T>>::RecGroupDependencies as List>::Concat<
            <Tail as HasTypeDependencies<TTypeListMarker<T>>>::RecGroupDependencies,
        >;
}

pub trait CompoundTypeDependencies<Fields, RGD> {
    type Dependencies: List;
    type RecGroupDependencies: List;
}

impl<Fields> CompoundTypeDependencies<Fields, ()> for TStruct<Fields>
where
    Fields: TRecGroupType<Vec<FieldType>, Rc<TypeRegistry>>
        + HasTypeDependencies<TTypeListMarker<FieldType>>,
    Fields::Dependencies: List,
{
    type Dependencies = <((HeapType, Self), ()) as List>::Concat<Fields::Dependencies>;

    type RecGroupDependencies = ();
}

impl<Fields, Head, Tail> CompoundTypeDependencies<Fields, (Head, Tail)> for TStruct<Fields>
where
    Fields: TRecGroupType<Vec<FieldType>, RecGroupRegistry>
        + HasTypeDependencies<TTypeListMarker<FieldType>>,
    Fields::RecGroupDependencies: List,
{
    type Dependencies = Fields::Dependencies;

    type RecGroupDependencies =
        <((HeapType, Self), ()) as List>::Concat<Fields::RecGroupDependencies>;
}

impl<Fields> HasTypeDependencies<HeapType> for TStruct<Fields>
where
    Fields: TRecGroupType<Vec<FieldType>, RecGroupRegistry>
        + HasTypeDependencies<TTypeListMarker<FieldType>>,
    Self: CompoundTypeDependencies<
            Fields,
            <Fields as HasTypeDependencies<TTypeListMarker<FieldType>>>::RecGroupDependencies,
        >,
{
    type Dependencies =
        <Self as CompoundTypeDependencies<Fields, Fields::RecGroupDependencies>>::Dependencies;

    type RecGroupDependencies = <Self as CompoundTypeDependencies<
        Fields,
        Fields::RecGroupDependencies,
    >>::RecGroupDependencies;
}

impl<Field> CompoundTypeDependencies<Field, ()> for TArray<Field>
where
    Field: TFieldType //TRecGroupType<FieldType, TypeRegistry>
        + HasTypeDependencies<FieldType, RecGroupDependencies = ()>,
    Field::ValType: TType<ValType>,
{
    type Dependencies = <((HeapType, Self), ()) as List>::Concat<Field::Dependencies>;

    type RecGroupDependencies = ();
}

impl<Field, Head, Tail> CompoundTypeDependencies<Field, (Head, Tail)> for TArray<Field>
where
    (Head, Tail): List,
    Field: TFieldType + HasTypeDependencies<FieldType, RecGroupDependencies = (Head, Tail)>,
{
    type Dependencies = Field::Dependencies;

    type RecGroupDependencies =
        <((HeapType, Self), ()) as List>::Concat<Field::RecGroupDependencies>;
}

impl<Field> HasTypeDependencies<HeapType> for TArray<Field>
where
    Field: TFieldType + HasTypeDependencies<FieldType>,
    Self: CompoundTypeDependencies<
            Field,
            <Field as HasTypeDependencies<FieldType>>::RecGroupDependencies,
        >,
{
    type Dependencies =
        <Self as CompoundTypeDependencies<Field, Field::RecGroupDependencies>>::Dependencies;

    type RecGroupDependencies = <Self as CompoundTypeDependencies<
        Field,
        Field::RecGroupDependencies,
    >>::RecGroupDependencies;
}

impl<Params, Results> CompoundTypeDependencies<(Params, Results), ()> for TFunc<Params, Results>
where
    Params: TRecGroupType<Vec<ValType>, RecGroupRegistry>
        + HasTypeDependencies<TTypeListMarker<ValType>>,
    Results: TRecGroupType<Vec<ValType>, RecGroupRegistry>
        + HasTypeDependencies<TTypeListMarker<ValType>>,
{
    type Dependencies =
        <<((HeapType, Self), ()) as List>::Concat<Params::Dependencies> as List>::Concat<
            Results::Dependencies,
        >;

    type RecGroupDependencies = ();
}

impl<Params, Results, Head, Tail> CompoundTypeDependencies<(Params, Results), (Head, Tail)>
    for TFunc<Params, Results>
where
    Params: TRecGroupType<Vec<ValType>, RecGroupRegistry>
        + HasTypeDependencies<TTypeListMarker<ValType>>,
    Results: TRecGroupType<Vec<ValType>, RecGroupRegistry>
        + HasTypeDependencies<TTypeListMarker<ValType>>,
{
    type Dependencies = <Params::Dependencies as List>::Concat<Results::Dependencies>;

    type RecGroupDependencies = <<((HeapType, Self), ()) as List>::Concat<
        Params::RecGroupDependencies,
    > as List>::Concat<Results::RecGroupDependencies>;
}

impl<Params, Results> HasTypeDependencies<HeapType> for TFunc<Params, Results>
where
    Params:
        TRecGroupType<Vec<ValType>, RecGroupRegistry> + HasTypeDependencies<TTypeListMarker<ValType>>,
    Results:
        TRecGroupType<Vec<ValType>, RecGroupRegistry> + HasTypeDependencies<TTypeListMarker<ValType>>,
    Self: CompoundTypeDependencies<
            (Params, Results),
            <<Params as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies as List>::Concat<<Results as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies>,
        >,
{
    type Dependencies =
        <Self as CompoundTypeDependencies<
            (Params, Results),
            <<Params as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies as List>::Concat<<Results as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies>,
        >>::Dependencies;
    type RecGroupDependencies =
        <Self as CompoundTypeDependencies<
            (Params, Results),
            <<Params as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies as List>::Concat<<Results as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies>,
        >>::RecGroupDependencies;
}
