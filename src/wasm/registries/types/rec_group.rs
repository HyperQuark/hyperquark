use core::marker::PhantomData;

use wasm_encoder::HeapType;

use super::TypeRegistry;
use super::dependencies::HasTypeDependencies;
use super::registration::TRecGroupType;
use crate::prelude::*;
use crate::registry::RegistryResult;
use crate::wasm::registries::types::{CompoundType, List, RegistryItem, TypeRegistryLike};

pub struct RecGroupRegistry {
    pub types: Rc<TypeRegistry>,
    pub rec_group_start: u32,
    pub main_rec_types_num: u32,
    pub rec_type_deps: RefCell<Vec<CompoundType>>,
}

impl TypeRegistryLike for RecGroupRegistry {
    fn register<N>(&self, ty: CompoundType) -> HQResult<N>
    where
        N: RegistryResult,
    {
        let idx = if let Some(idx) = self
            .types
            .registry()
            .borrow()
            .get_index_of(&RegistryItem::Type(ty.clone()))
        {
            idx
        } else if let Some((idx, _)) = self
            .rec_type_deps
            .borrow()
            .iter()
            .find_position(|other| other == &&ty)
        {
            idx
        } else {
            let deps_len = self.rec_type_deps.borrow().len();
            self.rec_type_deps.borrow_mut().push(ty);
            deps_len + (self.main_rec_types_num + self.rec_group_start) as usize
        };
        idx.try_into()
            .map_err(|_| make_hq_bug!("registry index out of bounds"))
    }
}

pub struct RecGroupMember<Marker, T>(PhantomData<(Marker, T)>);

impl<Marker, T, U> HasTypeDependencies<U> for RecGroupMember<Marker, T>
where
    T: HasTypeDependencies<U>,
{
    type Dependencies = T::Dependencies;
    type RecGroupDependencies = T::RecGroupDependencies;
}

impl<Marker, T> TRecGroupType<CompoundType, RecGroupRegistry> for RecGroupMember<Marker, T>
where
    T: TRecGroupType<CompoundType, RecGroupRegistry>,
{
    fn rec_group_ty(types: &RecGroupRegistry) -> HQResult<CompoundType> {
        T::rec_group_ty(types)
    }
}

pub trait RecGroupMarker {
    const NAME: &str;

    type Types: List;
}

pub trait IsRecGroupMember {
    type Marker: RecGroupMarker;
}

impl<Marker, T> IsRecGroupMember for RecGroupMember<Marker, T>
where
    Marker: RecGroupMarker,
{
    type Marker = Marker;
}

pub struct TRecGroupItem<const I: u32>;

impl<const I: u32> HasTypeDependencies<HeapType> for TRecGroupItem<I> {
    type Dependencies = ();
    type RecGroupDependencies = ((HeapType, Self), ());
}

impl<const I: u32> TRecGroupType<HeapType, RecGroupRegistry> for TRecGroupItem<I> {
    fn rec_group_ty(registering_info: &RecGroupRegistry) -> HQResult<HeapType> {
        Ok(HeapType::Concrete(registering_info.rec_group_start + I))
    }
}

#[macro_export]
macro_rules! rec_group {
    (
        $rec_group_name:ident {
            $($name:ident = $typename:ident{$($typeparams:tt)+};)+
        }
    ) => {
        mod $rec_group_name {
            use $crate::wasm::registries::types::*;
            use $crate::wasm::registries::types::dependencies::HasTypeDependencies;

            macro_rules! sub_rec_group_types {
                (
                    sub_rec_group_types!($$($$macro_args:tt)+)
                ) => {
                    sub_rec_group_types!($$($$macro_args)+)
                };
                (
                    $$ty:ident{$$({$$($$params:tt)+}),+}
                ) => {
                    $$ty<
                        $$(
                            sub_rec_group_types!(
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
                        sub_rec_group_types!(
                            $$($$first)+
                        ),
                        ()
                    )
                };
                (
                    ({$$($$first:tt)+}, $$({$$($$rest:tt)+}),+ $$(,)?)
                ) => {
                    (
                        sub_rec_group_types!(
                            $$($$first)+
                        ),
                        sub_rec_group_types!(
                            ($$({$$($$rest)+},)+)
                        )
                    )
                };
            }

            fn dummy_rec_group() -> Rc<RecGroup> {
                Rc::new(RecGroup {
                    name: stringify!($rec_group_name).into(),
                    types: vec![]
                })
            }

            fn register_rec_group(types: &Rc<TypeRegistry>) -> HQResult<()> {
                if types.registry().borrow().contains_key(&RegistryItem::RecGroupItem(dummy_rec_group(), 0)) {
                    return Ok(());
                }
                $(
                    <$name as HasTypeDependencies<HeapType>>::Dependencies::register_each(types)?;
                )+
                let start_index = types.registry().borrow().len() as u32;
                let rec_group_info = RecGroupRegistry {
                    types: Rc::clone(types),
                    rec_group_start: start_index,
                    main_rec_types_num: ${count($name)},
                    rec_type_deps: RefCell::new(vec![]),
                };
                let mut compound_types: Vec<CompoundType> = vec![];
                $(
                    compound_types.push($name::rec_group_ty(&rec_group_info)?);
                )+
                compound_types.extend(rec_group_info.rec_type_deps.take());
                let num_types = compound_types.len() as u32;
                let rec_group = Rc::new(RecGroup {
                    name: stringify!($rec_group_name).into(),
                    types: compound_types,
                });
                for i in 0..num_types {
                    types.register_default::<usize>(RegistryItem::RecGroupItem(
                        Rc::clone(&rec_group),
                        i
                    ))?;
                }
                Ok(())
            }

            pub struct Marker;

            impl RecGroupMarker for Marker {
                const NAME: &str = stringify!($rec_group_name);

                type Types = ty_list!($($name),+);
            }

            $(
                pub type $name = RecGroupMember<
                    Marker,
                    sub_rec_group_types!(
                        $typename{$($typeparams)+}
                    )
                >;

                // type ${concat($name, CompoundTypeRecGroupDependencies)} = <
                //     <<
                //         $name as $crate::wasm::registries::types::dependencies::HasTypeDependencies<HeapType>
                //     >::RecGroupDependencies as List>::Tail
                //     as $crate::wasm::registries::types::tyfp::Filter<$crate::wasm::registries::types::rec_group::HasCompoundTypeRegistration>
                // >::Filtered;

                impl TRecGroupType<u32, Rc<TypeRegistry>> for $name {
                    fn rec_group_ty(types: &Rc<TypeRegistry>) -> HQResult<u32> {
                        register_rec_group(types)?;
                        let rec_group_types = dummy_rec_group();
                        types.register_default(RegistryItem::RecGroupItem(rec_group_types, ${index()}))
                    }
                }
            )+
        }

        pub use $rec_group_name::{$($name),+};
    }
}
