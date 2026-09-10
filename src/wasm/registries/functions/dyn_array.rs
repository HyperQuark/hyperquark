use core::marker::PhantomData;

use wasm_encoder::{BlockType as WasmBlockType, ValType};
use wasm_gen::wasm_const;

use super::{MaybeStaticFunction, StaticFunction};
use crate::prelude::*;
use crate::wasm::registries::TypeRegistry;
use crate::wasm::registries::types::{
    TDefaultable, TDynArray, TDynArrayField, TNonNullable, TType,
};

#[derive(Clone)]
pub struct DynArrayFuncOverride {
    pub types: Rc<TypeRegistry>,
}

/// Pushes an element to a dynamic (resizeable) array
///
/// Takes 2 parameters:
/// ref `dynamic_array`<t> - the dynamic array struct (obtained from `TDynArray<T>` for `T: TDefaultable`)
/// t                    - the element
pub struct DynArrayPush<T>(PhantomData<T>);
impl<T: TType<ValType> + TDefaultable> NamedRegistryItem<MaybeStaticFunction>
    for DynArrayPush<T>
{
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}

impl<T: TType<ValType> + TDefaultable>
    TryNamedRegistryItemOverride<MaybeStaticFunction, DynArrayFuncOverride> for DynArrayPush<T>
{
    fn try_override(
        DynArrayFuncOverride { types }: DynArrayFuncOverride,
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
                params: Box::from([<TNonNullable<TDynArray<T>>>::ty(&types)?, T::ty(&types)?]),
                returns: Box::from([]),
                locals: Box::from([ValType::I32, <TNonNullable<TDynArrayField<T>>>::ty(&types)?]),
            }),
            maybe_populate: || None,
        })
    }
}

/// Gets an element of a dynamic (resizeable) array
///
/// Takes 2 parameters:
/// ref `dynamic_array`<t> - the dynamic array struct (obtained from `TDynArray<T>` for `T: TDefaultable`)
/// i32                  - the index
///
/// Returns t
pub struct DynArrayGet<T>(PhantomData<T>);
impl<T: TType<ValType> + TDefaultable> NamedRegistryItem<MaybeStaticFunction> for DynArrayGet<T> {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}

impl<T: TType<ValType> + TDefaultable>
    TryNamedRegistryItemOverride<MaybeStaticFunction, DynArrayFuncOverride> for DynArrayGet<T>
{
    fn try_override(
        DynArrayFuncOverride { types }: DynArrayFuncOverride,
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
                    LocalGet(1),
                    ArrayGet(array_type),
                ] as &[_]),
                params: Box::from([<TNonNullable<TDynArray<T>>>::ty(&types)?, ValType::I32]),
                returns: Box::from([T::ty(&types)?]),
                locals: Box::from([]),
            }),
            maybe_populate: || None,
        })
    }
}

/// Sets an element of a dynamic (resizeable) array
///
/// Takes 3 parameters:
/// ref `dynamic_array`<t> - the dynamic array struct (obtained from `TDynArray<T>` for `T: TDefaultable`)
/// i32                  - the index
/// t                    - the element
pub struct DynArraySet<T>(PhantomData<T>);
impl<T: TType<ValType> + TDefaultable> NamedRegistryItem<MaybeStaticFunction> for DynArraySet<T> {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}

impl<T: TType<ValType> + TDefaultable>
    TryNamedRegistryItemOverride<MaybeStaticFunction, DynArrayFuncOverride> for DynArraySet<T>
{
    fn try_override(
        DynArrayFuncOverride { types }: DynArrayFuncOverride,
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
                    LocalGet(1),
                    LocalGet(2),
                    ArrayGet(array_type),
                ] as &[_]),
                params: Box::from([
                    <TNonNullable<TDynArray<T>>>::ty(&types)?,
                    ValType::I32,
                    T::ty(&types)?,
                ]),
                returns: Box::from([T::ty(&types)?]),
                locals: Box::from([]),
            }),
            maybe_populate: || None,
        })
    }
}

/// Pops the last element from a dynamic (resizeable) array
///
/// Takes 1 parameter:
/// ref `dynamic_array`<t> - the dynamic array struct (obtained from `TDynArray<T>` for `T: TDefaultable`)
///
/// Returns t
pub struct DynArrayPop<T>(PhantomData<T>);
impl<T: TType<ValType> + TDefaultable> NamedRegistryItem<MaybeStaticFunction> for DynArrayPop<T> {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}

impl<T: TType<ValType> + TDefaultable>
    TryNamedRegistryItemOverride<MaybeStaticFunction, DynArrayFuncOverride> for DynArrayPop<T>
{
    fn try_override(
        DynArrayFuncOverride { types }: DynArrayFuncOverride,
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
                    LocalGet(0),
                    StructGet {
                        struct_type_index: struct_type,
                        field_index: 1,
                    },
                    I32Const(1),
                    I32Sub,
                    LocalTee(1),
                    ArrayGet(array_type),
                    LocalGet(0),
                    LocalGet(1),
                    StructSet {
                        struct_type_index: struct_type,
                        field_index: 1,
                    },
                ] as &[_]),
                params: Box::from([<TNonNullable<TDynArray<T>>>::ty(&types)?]),
                returns: Box::from([T::ty(&types)?]),
                locals: Box::from([ValType::I32]),
            }),
            maybe_populate: || None,
        })
    }
}

/// Creates a new dynamic (resizeable) array of the given capacity
///
/// Takes 1 parameter:
/// i32 - the initial capacity of the array to create
///
/// Returns ref `dynamic_array`<t>
pub struct DynArrayNew<T>(PhantomData<T>);
impl<T: TType<ValType> + TDefaultable> NamedRegistryItem<MaybeStaticFunction> for DynArrayNew<T> {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}

impl<T: TType<ValType> + TDefaultable>
    TryNamedRegistryItemOverride<MaybeStaticFunction, DynArrayFuncOverride> for DynArrayNew<T>
{
    fn try_override(
        DynArrayFuncOverride { types }: DynArrayFuncOverride,
    ) -> HQResult<MaybeStaticFunction> {
        let struct_type = types.register_comp::<TDynArray<T>, u32>()?;
        let array_type = types.register_comp::<TDynArrayField<T>, u32>()?;
        Ok(MaybeStaticFunction {
            static_function: Some(StaticFunction {
                export: None,
                instructions: Box::from(wasm_const![
                    LocalGet(0),
                    ArrayNewDefault(array_type),
                    I32Const(0),
                    StructNew(struct_type),
                ] as &[_]),
                params: Box::from([ValType::I32]),
                returns: Box::from([<TNonNullable<TDynArray<T>>>::ty(&types)?]),
                locals: Box::from([]),
            }),
            maybe_populate: || None,
        })
    }
}

/// Returns the length (not capacity) of the given dynamic array
///
/// Takes 1 parameter:
/// ref `dynamic_array`<t> - the dynamic array
///
/// Returns i32
pub struct DynArrayLen<T>(PhantomData<T>);
impl<T: TType<ValType> + TDefaultable> NamedRegistryItem<MaybeStaticFunction> for DynArrayLen<T> {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}

impl<T: TType<ValType> + TDefaultable>
    TryNamedRegistryItemOverride<MaybeStaticFunction, DynArrayFuncOverride> for DynArrayLen<T>
{
    fn try_override(
        DynArrayFuncOverride { types }: DynArrayFuncOverride,
    ) -> HQResult<MaybeStaticFunction> {
        let struct_type = types.register_comp::<TDynArray<T>, u32>()?;
        Ok(MaybeStaticFunction {
            static_function: Some(StaticFunction {
                export: None,
                instructions: Box::from(wasm_const![
                    LocalGet(0),
                    StructGet {
                        struct_type_index: struct_type,
                        field_index: 1
                    },
                ] as &[_]),
                params: Box::from([<TNonNullable<TDynArray<T>>>::ty(&types)?]),
                returns: Box::from([ValType::I32]),
                locals: Box::from([]),
            }),
            maybe_populate: || None,
        })
    }
}

/// Clears the given dynamic array to length 0 (but doesn't actually drop any of the elements)
///
/// Takes 1 parameter:
/// ref `dynamic_array`<t> - the dynamic array
pub struct DynArrayClear<T>(PhantomData<T>);
impl<T: TType<ValType> + TDefaultable> NamedRegistryItem<MaybeStaticFunction> for DynArrayClear<T> {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}

impl<T: TType<ValType> + TDefaultable>
    TryNamedRegistryItemOverride<MaybeStaticFunction, DynArrayFuncOverride> for DynArrayClear<T>
{
    fn try_override(
        DynArrayFuncOverride { types }: DynArrayFuncOverride,
    ) -> HQResult<MaybeStaticFunction> {
        let struct_type = types.register_comp::<TDynArray<T>, u32>()?;
        Ok(MaybeStaticFunction {
            static_function: Some(StaticFunction {
                export: None,
                instructions: Box::from(wasm_const![
                    LocalGet(0),
                    I32Const(0),
                    StructSet {
                        struct_type_index: struct_type,
                        field_index: 1
                    },
                ] as &[_]),
                params: Box::from([<TNonNullable<TDynArray<T>>>::ty(&types)?]),
                returns: Box::from([ValType::I32]),
                locals: Box::from([]),
            }),
            maybe_populate: || None,
        })
    }
}
