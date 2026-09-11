use wasm_encoder::{BlockType as WasmBlockType, ValType};
use wasm_gen::wasm_const;

use super::{MaybeStaticFunction, StaticFunction};
use crate::prelude::*;
use crate::wasm::registries::StaticFunctionRegistry;
use crate::wasm::registries::functions::StaticFunctionRegistrar;
use crate::wasm::registries::functions::dyn_array::{DynArrayGet, DynArrayLen};
use crate::wasm::registries::types::{
    TDynArray, TNonNullable, TNullable, TStackArray, TStackStruct, TStepFunc, TTargetThreadArray,
    TTargetThreadsStruct, TThreadArray, TType,
};

pub struct Tick;
impl NamedRegistryItem<MaybeStaticFunction> for Tick {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        register_deps: || {
            vec![
                StaticFunctionRegistry::registration::<DynArrayLen<TNullable<TStackArray>>>(),
                StaticFunctionRegistry::registration::<DynArrayGet<TNullable<TStackArray>>>(),
                StaticFunctionRegistry::registration::<DynArrayLen<TNullable<TStackStruct>>>(),
                StaticFunctionRegistry::registration::<DynArrayGet<TNullable<TStackStruct>>>(),
            ]
        },
        maybe_populate: |proj, static_functions| {
            let types = Rc::clone(proj.registries().types());

            let stack_struct_type = types.register_comp::<TStackStruct, _>()?;
            let target_thread_struct_type = types.register_comp::<TTargetThreadsStruct, _>()?;
            let target_threads_array_type = types.register_comp::<TTargetThreadArray, _>()?;
            let step_func_ty = types.register_comp::<TStepFunc, _>()?;

            // this is fine to use here because strings are finished before static funcs,
            // and strings are the only imported globals.
            let imported_globals = proj.imported_global_count()?;

            let threadss_global = proj.threadss_global::<u32>()? + imported_globals;

            let targets_num = 1 + proj.costume_names().len() as i32;

            let imported_func_count = proj.imported_func_count()?;

            hq_assert!(targets_num > 0);

            const LOCAL_TARGET_INDEX: u32 = 0;
            const LOCAL_STACK_INDEX: u32 = 1;
            const LOCAL_THREADS_NUM: u32 = 2;
            const LOCAL_THREAD_LIST: u32 = 3;
            const LOCAL_THREAD: u32 = 4;
            const LOCAL_STEP: u32 = 5;

            Ok(Some(StaticFunction {
                export: Some("tick".into()),
                instructions: Box::from(wasm_const![
                    Loop(WasmBlockType::Empty),
                    GlobalGet(threadss_global),
                    LocalGet(LOCAL_TARGET_INDEX),
                    ArrayGet(target_threads_array_type),
                    StructGet {
                        struct_type_index: target_thread_struct_type,
                        field_index: 1,
                    },
                    LocalTee(LOCAL_THREAD_LIST),
                    Call(
                        imported_func_count
                            + static_functions
                                .get_index_of(&StaticFunctionRegistrar::name::<
                                    DynArrayLen<TNullable<TStackArray>>,
                                >())
                                .ok_or_else(|| make_hq_bug!(
                                    "static function dependency not registered"
                                ))? as u32
                    ),
                    LocalTee(LOCAL_THREADS_NUM),
                    I32Eqz,
                    BrIf(0),
                    I32Const(0),
                    LocalSet(LOCAL_STACK_INDEX),
                    Loop(WasmBlockType::Empty),
                    LocalGet(LOCAL_THREAD_LIST),
                    LocalGet(LOCAL_STACK_INDEX),
                    Call(
                        imported_func_count
                            + static_functions
                                .get_index_of(&StaticFunctionRegistrar::name::<
                                    DynArrayGet<TNullable<TStackArray>>,
                                >())
                                .ok_or_else(|| make_hq_bug!(
                                    "static function dependency not registered"
                                ))? as u32
                    ),
                    RefAsNonNull,
                    LocalTee(LOCAL_THREAD),
                    LocalGet(LOCAL_THREAD),
                    RefCastNonNull(<TDynArray<TNullable<TStackStruct>>>::ty(&types)?),
                    LocalGet(LOCAL_THREAD),
                    RefCastNonNull(<TDynArray<TNullable<TStackStruct>>>::ty(&types)?),
                    Call(
                        imported_func_count
                            + static_functions
                                .get_index_of(&StaticFunctionRegistrar::name::<
                                    DynArrayLen<TNullable<TStackStruct>>,
                                >())
                                .ok_or_else(|| make_hq_bug!(
                                    "static function dependency not registered"
                                ))? as u32
                    ),
                    I32Const(1),
                    I32Sub,
                    Call(
                        imported_func_count
                            + static_functions
                                .get_index_of(&StaticFunctionRegistrar::name::<
                                    DynArrayGet<TNullable<TStackStruct>>,
                                >())
                                .ok_or_else(|| make_hq_bug!(
                                    "static function dependency not registered"
                                ))? as u32
                    ),
                    RefAsNonNull,
                    LocalTee(LOCAL_STEP),
                    StructGet {
                        struct_type_index: stack_struct_type,
                        field_index: 1,
                    },
                    LocalGet(LOCAL_STEP),
                    StructGet {
                        struct_type_index: stack_struct_type,
                        field_index: 0,
                    },
                    CallRef(step_func_ty),
                    LocalGet(LOCAL_STACK_INDEX),
                    I32Const(1),
                    I32Add,
                    LocalTee(LOCAL_STACK_INDEX),
                    LocalGet(LOCAL_THREADS_NUM),
                    I32LtS,
                    BrIf(0),
                    End,
                    LocalGet(LOCAL_TARGET_INDEX),
                    I32Const(1),
                    I32Add,
                    LocalTee(LOCAL_TARGET_INDEX),
                    I32Const(targets_num),
                    I32LtS,
                    BrIf(0),
                    End,
                    End,
                ] as &[_]),
                params: Box::new([]),
                returns: Box::new([]),
                locals: Box::new([
                    ValType::I32,
                    ValType::I32,
                    ValType::I32,
                    <TNonNullable<TThreadArray>>::ty(&types)?,
                    <TNonNullable<TStackArray>>::ty(&types)?,
                    <TNonNullable<TStackStruct>>::ty(&types)?,
                ]),
            }))
        },
    };
}
