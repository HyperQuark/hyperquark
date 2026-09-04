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

trait TypeRegisteringInfo {
    fn types(&self) -> &TypeRegistry;
}

#[derive(Clone)]
struct RecGroupInfo {
    types: Rc<TypeRegistry>,
    rec_group_start: u32,
}

impl<'a> TypeRegisteringInfo for RecGroupInfo {
    fn types(&self) -> &TypeRegistry {
        &self.types
    }
}

impl TypeRegisteringInfo for TypeRegistry {
    fn types(&self) -> &TypeRegistry {
        &self
    }
}

pub impl(self) trait TRecGroupType<T, I: TypeRegisteringInfo> {
    fn rec_group_ty(registering_info: &I) -> HQResult<T>;
}

impl<T, I> TRecGroupType<HeapType, I> for T
where
    T: TRecGroupType<u32, I>,
    I: TypeRegisteringInfo,
{
    default fn rec_group_ty(types: &I) -> HQResult<HeapType> {
        Ok(HeapType::Concrete(T::rec_group_ty(types)?))
    }
}

pub trait TType<T> {
    fn ty(types: &TypeRegistry) -> HQResult<T>;
}

impl<T, U> TType<T> for U
where
    U: TRecGroupType<T, TypeRegistry>,
{
    fn ty(types: &TypeRegistry) -> HQResult<T> {
        U::rec_group_ty(types)
    }
}

impl<T> CompTimeRegistrand<TypeRegistry, u32> for T
where
    T: TType<u32>,
{
    fn register(types: &TypeRegistry) -> HQResult<u32> {
        T::ty(types)
    }
}

trait HasTypeDependencies<T> {
    type Dependencies: TypeList;
    type RecGroupDependencies: TypeList;
}

trait TypeList {
    type Head;
    type Tail: TypeList;

    type Concat<Other: TypeList>: TypeList;
}
trait RegTypeList<I>: TypeList {
    fn register_each(types: &I) -> HQResult<()>;
}

impl TypeList for () {
    type Head = ();
    type Tail = ();

    type Concat<Other: TypeList> = Other;
}

impl<I> RegTypeList<I> for () {
    fn register_each(_types: &I) -> HQResult<()> {
        Ok(())
    }
}

impl<HeadT, Head> TypeList for ((HeadT, Head),) {
    type Head = (HeadT, Head);
    type Tail = ();

    type Concat<Other: TypeList> = ((HeadT, Head), Other);
}

impl<HeadT, Head, Tail> TypeList for ((HeadT, Head), Tail)
where
    Tail: TypeList,
{
    type Head = (HeadT, Head);
    type Tail = Tail;

    type Concat<Other: TypeList> = ((HeadT, Head), Tail::Concat<Other>);
}

impl<HeadT, Head, Tail, I> RegTypeList<I> for ((HeadT, Head), Tail)
where
    I: TypeRegisteringInfo,
    Head: TRecGroupType<HeadT, I>,
    Tail: RegTypeList<I>,
{
    fn register_each(types: &I) -> HQResult<()> {
        Head::rec_group_ty(types)?;
        Tail::register_each(types)
    }
}

// impl<'a, T, U> TRecGroupType<T, RecGroupInfo> for U
// where
//     U: TType<T>,
// {
//     fn rec_group_ty(registering_info: &RecGroupInfo) -> HQResult<T> {
//         <U as TType<T>>::ty(registering_info.types)
//     }
// }

pub struct TStructRef;
impl<I: TypeRegisteringInfo> TRecGroupType<u32, I> for TStructRef {
    fn rec_group_ty(_types: &I) -> HQResult<u32> {
        panic!("this shouldn't be called ever!!! evil!!!")
    }
}
impl<I: TypeRegisteringInfo> TRecGroupType<HeapType, I> for TStructRef {
    fn rec_group_ty(_types: &I) -> HQResult<HeapType> {
        Ok(HeapType::Abstract {
            shared: false,
            ty: AbstractHeapType::Struct,
        })
    }
}
impl HasTypeDependencies<HeapType> for TStructRef {
    type Dependencies = ();
    type RecGroupDependencies = ();
}

pub trait TRefType {
    type HeapType;
    const NULLABLE: bool;
}

impl<T, I> TRecGroupType<RefType, I> for T
where
    T: TRefType,
    T::HeapType: TRecGroupType<HeapType, I>,
    I: TypeRegisteringInfo,
{
    fn rec_group_ty(types: &I) -> HQResult<RefType> {
        Ok(RefType {
            nullable: T::NULLABLE,
            heap_type: T::HeapType::rec_group_ty(types)?,
        })
    }
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
    I: TypeRegisteringInfo,
{
    fn rec_group_ty(types: &I) -> HQResult<ValType> {
        Ok(ValType::Ref(
            <T as TRecGroupType<RefType, I>>::rec_group_ty(types)?,
        ))
    }
}

pub struct TI32;

impl<I: TypeRegisteringInfo> TRecGroupType<ValType, I> for TI32 {
    fn rec_group_ty(_types: &I) -> HQResult<ValType> {
        Ok(ValType::I32)
    }
}
impl TDefaultable for TI32 {}
impl HasTypeDependencies<ValType> for TI32 {
    type Dependencies = ();
    type RecGroupDependencies = ();
}

pub trait TFieldType {
    type ValType;
    const MUTABLE: bool;
}

impl<T, I> TRecGroupType<FieldType, I> for T
where
    T: TFieldType,
    I: TypeRegisteringInfo,
    T::ValType: TRecGroupType<ValType, I>,
{
    fn rec_group_ty(types: &I) -> HQResult<FieldType> {
        Ok(FieldType {
            element_type: StorageType::Val(T::ValType::rec_group_ty(types)?),
            mutable: T::MUTABLE,
        })
    }
}

impl<T> HasTypeDependencies<FieldType> for T
where
    T: TFieldType,
    T::ValType: HasTypeDependencies<ValType>,
{
    type Dependencies = <T::ValType as HasTypeDependencies<ValType>>::Dependencies;
    type RecGroupDependencies = <T::ValType as HasTypeDependencies<ValType>>::RecGroupDependencies;
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

pub trait TDefaultable {}

impl<T, I: TypeRegisteringInfo> TRecGroupType<Vec<T>, I> for () {
    fn rec_group_ty(_types: &I) -> HQResult<Vec<T>> {
        Ok(vec![])
    }
}

impl<T, I, Head, Tail> TRecGroupType<Vec<T>, I> for (Head, Tail)
where
    I: TypeRegisteringInfo,
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

impl<Fields, I> TRecGroupType<u32, I> for TStruct<Fields>
where
    I: TypeRegisteringInfo,
    Fields: TRecGroupType<Vec<FieldType>, I>,
{
    fn rec_group_ty(types: &I) -> HQResult<u32> {
        types.types().struct_(Fields::rec_group_ty(types)?)
    }
}

struct TTypeListMarker<T>(PhantomData<T>);

impl<T> HasTypeDependencies<TTypeListMarker<T>> for () {
    type Dependencies = ();
    type RecGroupDependencies = ();
}

impl<T, Head, Tail> HasTypeDependencies<TTypeListMarker<T>> for (Head, Tail)
where
    Head: HasTypeDependencies<T>,
    Head::Dependencies: TypeList,
    Head::RecGroupDependencies: TypeList,
    Tail: HasTypeDependencies<TTypeListMarker<T>>,
    Tail::Dependencies: TypeList,
    Tail::RecGroupDependencies: TypeList,
{
    type Dependencies = <<Head as HasTypeDependencies<T>>::Dependencies as TypeList>::Concat<
        <Tail as HasTypeDependencies<TTypeListMarker<T>>>::Dependencies,
    >;
    type RecGroupDependencies =
        <<Head as HasTypeDependencies<T>>::RecGroupDependencies as TypeList>::Concat<
            <Tail as HasTypeDependencies<TTypeListMarker<T>>>::RecGroupDependencies,
        >;
}

trait CompoundTypeDependencies<Fields, RGD> {
    type Dependencies: TypeList;
    type RecGroupDependencies: TypeList;
}

impl<Fields> CompoundTypeDependencies<Fields, ()> for TStruct<Fields>
where
    Fields: TRecGroupType<Vec<FieldType>, TypeRegistry>
        + HasTypeDependencies<TTypeListMarker<FieldType>>,
    Fields::Dependencies: TypeList,
{
    type Dependencies = <((HeapType, Self),) as TypeList>::Concat<Fields::Dependencies>;

    type RecGroupDependencies = ();
}

impl<Fields, Head, Tail> CompoundTypeDependencies<Fields, (Head, Tail)> for TStruct<Fields>
where
    Fields: TRecGroupType<Vec<FieldType>, RecGroupInfo>
        + HasTypeDependencies<TTypeListMarker<FieldType>>,
    Fields::RecGroupDependencies: TypeList,
{
    type Dependencies = Fields::Dependencies;

    type RecGroupDependencies =
        <((HeapType, Self),) as TypeList>::Concat<Fields::RecGroupDependencies>;
}

impl<Fields> HasTypeDependencies<HeapType> for TStruct<Fields>
where
    Fields: TRecGroupType<Vec<FieldType>, RecGroupInfo>
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

pub struct TArray<Field>(PhantomData<Field>);

impl<Field, I> TRecGroupType<u32, I> for TArray<Field>
where
    I: TypeRegisteringInfo,
    Field: TFieldType,
    Field::ValType: TRecGroupType<ValType, I>,
{
    fn rec_group_ty(types: &I) -> HQResult<u32> {
        types.types().array(
            StorageType::Val(Field::ValType::rec_group_ty(types)?),
            Field::MUTABLE,
        )
    }
}

impl<Field> CompoundTypeDependencies<Field, ()> for TArray<Field>
where
    Field: TFieldType //TRecGroupType<FieldType, TypeRegistry>
        + HasTypeDependencies<FieldType, RecGroupDependencies = ()>,
    Field::ValType: TType<ValType>,
{
    type Dependencies = <((HeapType, Self),) as TypeList>::Concat<Field::Dependencies>;

    type RecGroupDependencies = ();
}

impl<Field, Head, Tail> CompoundTypeDependencies<Field, (Head, Tail)> for TArray<Field>
where
    (Head, Tail): TypeList,
    Field: TFieldType + HasTypeDependencies<FieldType, RecGroupDependencies = (Head, Tail)>,
{
    type Dependencies = Field::Dependencies;

    type RecGroupDependencies =
        <((HeapType, Self),) as TypeList>::Concat<Field::RecGroupDependencies>;
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

pub struct TFunc<Params, Result>(PhantomData<Params>, PhantomData<Result>);

impl<Params, Result, I> TRecGroupType<u32, I> for TFunc<Params, Result>
where
    I: TypeRegisteringInfo,
    Params: TRecGroupType<Vec<ValType>, I>,
    Result: TRecGroupType<Vec<ValType>, I>,
{
    fn rec_group_ty(types: &I) -> HQResult<u32> {
        types
            .types()
            .function(Params::rec_group_ty(types)?, Result::rec_group_ty(types)?)
    }
}

impl<Params, Results> CompoundTypeDependencies<(Params, Results), ()> for TFunc<Params, Results>
where
    Params:
        TRecGroupType<Vec<ValType>, RecGroupInfo> + HasTypeDependencies<TTypeListMarker<ValType>>,
    Results:
        TRecGroupType<Vec<ValType>, RecGroupInfo> + HasTypeDependencies<TTypeListMarker<ValType>>,
{
    type Dependencies =
        <<((HeapType, Self),) as TypeList>::Concat<Params::Dependencies> as TypeList>::Concat<
            Results::Dependencies,
        >;

    type RecGroupDependencies = ();
}

impl<Params, Results, Head, Tail> CompoundTypeDependencies<(Params, Results), (Head, Tail)>
    for TFunc<Params, Results>
where
    Params:
        TRecGroupType<Vec<ValType>, RecGroupInfo> + HasTypeDependencies<TTypeListMarker<ValType>>,
    Results:
        TRecGroupType<Vec<ValType>, RecGroupInfo> + HasTypeDependencies<TTypeListMarker<ValType>>,
{
    type Dependencies = <Params::Dependencies as TypeList>::Concat<Results::Dependencies>;

    type RecGroupDependencies = <<((HeapType, Self),) as TypeList>::Concat<
        Params::RecGroupDependencies,
    > as TypeList>::Concat<Results::RecGroupDependencies>;
}

impl<Params, Results> HasTypeDependencies<HeapType> for TFunc<Params, Results>
where
    Params:
        TRecGroupType<Vec<ValType>, RecGroupInfo> + HasTypeDependencies<TTypeListMarker<ValType>>,
    Results:
        TRecGroupType<Vec<ValType>, RecGroupInfo> + HasTypeDependencies<TTypeListMarker<ValType>>,
    Self: CompoundTypeDependencies<
            (Params, Results),
            <<Params as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies as TypeList>::Concat<<Results as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies>,
        >,
{
    type Dependencies =
        <Self as CompoundTypeDependencies<
            (Params, Results),
            <<Params as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies as TypeList>::Concat<<Results as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies>,
        >>::Dependencies;
    type RecGroupDependencies =
        <Self as CompoundTypeDependencies<
            (Params, Results),
            <<Params as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies as TypeList>::Concat<<Results as HasTypeDependencies<TTypeListMarker<ValType>>>::RecGroupDependencies>,
        >>::RecGroupDependencies;
}

macro_rules! rec_group {
    (
        $rec_group_name:ident {
            $($name:ident = $typename:ident{$($typeparams:tt)+};)+
        }
    ) => {
        macro_rules! ${ concat($rec_group_name, _sub_rec_group_types) } {
            (
                ${concat($rec_group_name, _sub_rec_group_types)}!($$($$macro_args:tt)+)
            ) => {
                ${concat($rec_group_name, _sub_rec_group_types)}!($$($$macro_args)+)
            };
            (
                $$ty:ident{$$({$$($$params:tt)+}),+}
            ) => {
                $$ty<
                    $$(
                        ${concat($rec_group_name, _sub_rec_group_types)}!(
                            $$($$params)+
                        )
                    ),+
                >
            };
            $(
                ($name) => {
                    TRecGroupItem<${ index() }>
                };
            )+
            ($$ty:ident) => {
                $$ty
            };
            (()) => {()};
            (
                ({$$($$first:tt)+},)
            ) => {
                (
                    ${concat($rec_group_name, _sub_rec_group_types)}!(
                        $$($$first)+
                    ),
                    ()
                )
            };
            (
                ({$$($$first:tt)+}, $$({$$($$rest:tt)+}),+ $$(,)?)
            ) => {
                (
                    ${concat($rec_group_name, _sub_rec_group_types)}!(
                        $$($$first)+
                    ),
                    ${concat($rec_group_name, _sub_rec_group_types)}!(
                        ($$({$$($$rest)+},)+)
                    )
                )
            };
        }

        fn ${ concat($rec_group_name, _register_deps) }(types: &TypeRegistry) -> HQResult<()> {
            $(
                <${concat($name, Type)} as HasTypeDependencies<HeapType>>::Dependencies::register_each(types)?;
            )+
            Ok(())
        }

        $(
            type ${concat($name, Type)} = ${ concat($rec_group_name, _sub_rec_group_types) }!(
                $typename{$($typeparams)+}
            );

            pub struct $name;

            impl CompTimeRegistrand<TypeRegistry, u32> for $name {
                fn register(types: &TypeRegistry) -> HQResult<u32> {
                    ${ concat($rec_group_name, _register_deps) }(types)?;
                    hq_todo!()
                    // let rec_group_info = RecGroupInfo {
                    //     types,
                    //     rec_group_start: types.registry().len() as u32,
                    // };
                    // $(
                    //     $name::rec_group_ty(&rec_group_info)?;
                    // )

                }
            }
        )+
    }
}

pub struct TRecGroupItem<const I: u32>;

impl<const I: u32> HasTypeDependencies<HeapType> for TRecGroupItem<I> {
    type Dependencies = ();
    type RecGroupDependencies = ((HeapType, Self), ());
}

impl<'a, const I: u32> TRecGroupType<HeapType, RecGroupInfo> for TRecGroupItem<I> {
    fn rec_group_ty(registering_info: &RecGroupInfo) -> HQResult<HeapType> {
        Ok(HeapType::Concrete(registering_info.rec_group_start + I))
    }
}

rec_group! {
    rec_grp {
        TStepFunc = TFunc{
            {(
                {TNonNullable{{TStackArray}}},
                {TNullable{{TStructRef}}},
            )},
            {()}
        };
        TStackStruct = TStruct{{(
            {TMutField{{TNonNullable{{TStepFunc}}}}},
            {TConstField{{TNullable{{TStructRef}}}}},
        )}};
        TStackDynArrayField = TArray{{TMutField{{TNullable{{TStackStruct}}}}}};
        TStackArray = TDynArray{{TNullable{{TStackStruct}}}};
    }
}

pub type TDynArrayField<T> = TArray<TMutField<T>>;
pub type TDynArray<T> = TStruct<(
    TMutField<TNonNullable<TDynArrayField<T>>>,
    (TMutField<TI32>, ()),
)>;

pub type TThreadArray = TDynArray<TNullable<TStackArray>>;

pub type TTargetThreadsStruct =
    TStruct<(TMutField<TI32>, (TMutField<TNonNullable<TThreadArray>>, ()))>;

pub type TTargetThreadArray = TArray<TMutField<TNonNullable<TTargetThreadsStruct>>>;
