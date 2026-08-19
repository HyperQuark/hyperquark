
use wasm_encoder::{BlockType as WasmBlockType, ValType};
use wasm_gen::wasm_const;

use core::marker::PhantomData;

use super::{MaybeStaticFunction, StaticFunction};
use crate::{prelude::*, wasm::registries::{TypeRegistry, types::{TDynArray, TDynArrayField, TValType}}};

/// Pushes an element to a dynamic (resizeable) array
///
/// Takes 2 parameters:
/// ref dynamic_array<t> - the array
/// t                    - the element
pub struct DynArrayPush<T>(PhantomData<T>);
impl<T: TValType> NamedRegistryItem<MaybeStaticFunction> for DynArrayPush<T> {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}

pub struct DynArrayPushOverride {
    types: Rc<TypeRegistry>,
}

impl<T: TValType> TryNamedRegistryItemOverride<MaybeStaticFunction, DynArrayPushOverride>
    for DynArrayPush<T>
{
    fn try_override(
        DynArrayPushOverride { types }: DynArrayPushOverride,
    ) -> HQResult<MaybeStaticFunction> {
        let struct_type = types.register_comp::<TDynArray<T>, u32>()?;
        let array_type = types.register_comp::<TDynArrayField<T>, u32>()?;
        Ok(MaybeStaticFunction {
            static_function: Some(StaticFunction {
                export: None,
                instructions: Box::from(wasm_const![
                    LocalGet(0),
                    StructGet {
                        struct_type_index: struct_type,
                        field_index: 0,
                    },
                    ArrayLen,
                    LocalGet(0),
                    StructGet {
                        struct_type_index: struct_type,
                        field_index: 1,
                    },
                    LocalTee(2),
                    I32Eq,
                    If(WasmBlockType::Empty),
                        LocalGet(2),
                        I32Const(1),
                        I32Shl,
                        ArrayNewDefault(array_type), // dest
                        LocalTee(3),
                        I32Const(0), // dest index
                        LocalGet(0),
                        StructGet {
                            struct_type_index: struct_type,
                            field_index: 0,
                        }, // src
                        I32Const(0), // src index
                        LocalGet(2), // length
                        ArrayCopy {
                            array_type_index_dst: array_type,
                            array_type_index_src: array_type,
                        },
                        LocalGet(0),
                        LocalGet(3),
                        StructSet {
                            struct_type_index: struct_type,
                            field_index: 0,
                        },
                    End,
                    LocalGet(0),
                    StructGet {
                        struct_type_index: struct_type,
                        field_index: 0,
                    },
                    LocalGet(2),
                    LocalGet(1),
                    ArraySet(array_type),
                    LocalGet(0),
                    LocalGet(2),
                    I32Const(1),
                    I32Add,
                    StructSet {
                        struct_type_index: struct_type,
                        field_index: 1,
                    },
                ] as &[_]),
                params: Box::from([
                    TypeRegistry::ref_val(struct_type, false),
                    T::val_type(&types)?,
                ]),
                returns: Box::from([]),
                locals: Box::from([
                    ValType::I32,
                    TypeRegistry::ref_val(array_type, false),
                ]),
            }),
            maybe_populate: || None,
        })
    }
}

