use core::marker::PhantomData;

use wasm_encoder::HeapType;

use super::dependencies::HasTypeDependencies;
use super::registration::TRecGroupType;
use super::tyfp::{Func, Bool};
use super::{TypeRegisteringInfo, TypeRegistry};
use crate::prelude::*;
use crate::wasm::registries::types::CompoundType;

#[derive(Clone)]
pub struct RecGroupInfo {
    pub types: Rc<TypeRegistry>,
    pub rec_group_start: u32,
}

impl TypeRegisteringInfo for RecGroupInfo {
    fn types(&self) -> &TypeRegistry {
        &self.types
    }
}

#[macro_export]
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

        fn ${concat($rec_group_name, _construct_rec_group)}() -> Rc<RecGroup> {
            Rc::new(RecGroup {
                types: vec![
                    $(
                        (
                            core::any::type_name::<$name>(),
                            RefCell::new(CompoundType::Struct(vec![])),
                        )
                    ),+
                ]
            })
        }

        fn ${ concat($rec_group_name, _register_rec_group) }(types: &Rc<TypeRegistry>) -> HQResult<()> {
            use $crate::wasm::registries::types::dependencies::*;
            let rec_group_types = ${concat($rec_group_name, _construct_rec_group)}();
            if types.registry().borrow().contains_key(&RegistryItem::RecGroupItem(Rc::clone(&rec_group_types), 0)) {
                return Ok(());
            }
            $(
                <$name as HasTypeDependencies<HeapType>>::Dependencies::register_each(types)?;
            )+
            for i in 0u32..(rec_group_types.types.len() as u32) {
                types.register_default::<u32>(RegistryItem::RecGroupItem(Rc::clone(&rec_group_types), i))?;
            }
            let start_index = types.register_default::<u32>(RegistryItem::RecGroupItem(Rc::clone(&rec_group_types), 0))?;
            let rec_group_info = RecGroupInfo {
                types: Rc::clone(types),
                rec_group_start: start_index,
            };
            $(
                *rec_group_types.types[${index()}].1.borrow_mut() =
                    $name::rec_group_ty(
                        &rec_group_info
                    )?;
            )+
            Ok(())
        }

        $(
            pub type $name = ${ concat($rec_group_name, _sub_rec_group_types) }!(
                $typename{$($typeparams)+}
            );

            // type ${concat($name, CompoundTypeRecGroupDependencies)} = <
            //     <<
            //         $name as $crate::wasm::registries::types::dependencies::HasTypeDependencies<HeapType>
            //     >::RecGroupDependencies as List>::Tail
            //     as $crate::wasm::registries::types::tyfp::Filter<$crate::wasm::registries::types::rec_group::HasCompoundTypeRegistration>
            // >::Filtered;

            impl TRecGroupType<u32, Rc<TypeRegistry>> for $name {
                fn rec_group_ty(types: &Rc<TypeRegistry>) -> HQResult<u32> {
                    ${ concat($rec_group_name, _register_rec_group) }(types)?;
                    let rec_group_types = ${concat($rec_group_name, _construct_rec_group)}();
                    types.register_default(RegistryItem::RecGroupItem(rec_group_types, ${index()}))
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

impl<const I: u32> TRecGroupType<HeapType, RecGroupInfo> for TRecGroupItem<I> {
    fn rec_group_ty(registering_info: &RecGroupInfo) -> HQResult<HeapType> {
        Ok(HeapType::Concrete(registering_info.rec_group_start + I))
    }
}

pub struct HasCompoundTypeRegistration;
impl Func for HasCompoundTypeRegistration {
    type Func<T> = CompoundTypeRegistrationTester<T>;
}

pub struct CompoundTypeRegistrationTester<T>(PhantomData<T>);

impl<T> Bool for CompoundTypeRegistrationTester<T> {
    default const BOOL: bool = false;
}

impl<HeadT, Head> Bool for CompoundTypeRegistrationTester<(HeadT, Head)>
where
    Head: TRecGroupType<CompoundType, RecGroupInfo>,
{
    const BOOL: bool = true;
}
