use core::marker::PhantomData;

use wasm_encoder::HeapType;

use super::TypeRegistry;
use super::dependencies::HasTypeDependencies;
use super::registration::TRecGroupType;
use super::tyfp::{Bool, Func};
use crate::prelude::*;
use crate::wasm::registries::types::{CompoundType, RegistryItem, TypeRegistryLike};

pub struct RecGroupRegistry {
    pub types: Rc<TypeRegistry>,
    pub rec_group_start: u32,
    pub main_rec_types_num: u32,
    pub rec_type_deps: RefCell<Vec<CompoundType>>,
}

impl TypeRegistryLike for RecGroupRegistry {
    fn register<N>(&self, ty: CompoundType) -> HQResult<N>
    where
        N: crate::registry::RegistryResult,
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

        fn ${concat($rec_group_name, _dummy_rec_group)}() -> Rc<RecGroup> {
            Rc::new(RecGroup {
                name: stringify!($rec_group_name).into(),
                types: vec![]
            })
        }

        fn ${ concat($rec_group_name, _register_rec_group) }(types: &Rc<TypeRegistry>) -> HQResult<()> {
            use $crate::wasm::registries::types::dependencies::*;
            use $crate::wasm::registries::types::rec_group::RecGroupRegistry;
            // let rec_group_types = ${concat($rec_group_name, _construct_rec_group)}();
            if types.registry().borrow().contains_key(&RegistryItem::RecGroupItem(${concat($rec_group_name, _dummy_rec_group)}(), 0)) {
                return Ok(());
            }
            $(
                <$name as HasTypeDependencies<HeapType>>::Dependencies::register_each(types)?;
            )+
            // for i in 0u32..(rec_group_types.types.len() as u32) {
            //     types.register_default::<u32>(RegistryItem::RecGroupItem(Rc::clone(&rec_group_types), i))?;
            // }
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
                    let rec_group_types = ${concat($rec_group_name, _dummy_rec_group)}();
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

impl<const I: u32> TRecGroupType<HeapType, RecGroupRegistry> for TRecGroupItem<I> {
    fn rec_group_ty(registering_info: &RecGroupRegistry) -> HQResult<HeapType> {
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
    Head: TRecGroupType<CompoundType, RecGroupRegistry>,
{
    const BOOL: bool = true;
}
