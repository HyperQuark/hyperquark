use wasm_encoder::{BlockType as WasmBlockType, HeapType, MemArg, ValType};
use wasm_gen::wasm_const;

use super::{MaybeStaticFunction, StaticFunction};
use crate::prelude::*;
use crate::wasm::mem_layout;
use crate::wasm::registries::functions::dyn_array::{DynArrayNew, DynArrayPop, DynArrayPush};
use crate::wasm::registries::types::{
    TNonNullable, TNullable, TStackArray, TStackStruct, TStepFunc, TTargetThreadArray,
    TThreadArray, TType,
};
use crate::wasm::registries::{GlobalRegistry, StaticFunctionRegistry, TypeRegistry};

#[derive(Clone)]
pub struct SpawnThreadFuncOverride {
    pub types: Rc<TypeRegistry>,
    pub globals: Rc<GlobalRegistry>,
    pub static_functions: Rc<StaticFunctionRegistry>,
    pub num_sprites: u32,
    pub imported_func_count: u32,
    pub imported_global_count: u32,
}

type StackStructRef = TNullable<TStackStruct>;

/// Spawns a new thread in the same stack (i.e. a thread that yields back to the current
/// thread once it completes).
/// 
/// The step that is provided to return to will be written into
/// the current stack frame, and the new thread's step is added to the top of the current
/// frame with the provided struct argument so that that will run until completion before
/// yielding to the provided next step.
///
/// Takes 4 parameters:
/// - ref `stack_array`` - the current stack
/// - ref `step_func`` - the step to spawn
/// - structref - the structref to pass to the step being spawned
/// - ref `step_func`` - the step to return to after
pub struct SpawnThreadInStack;
impl NamedRegistryItem<MaybeStaticFunction> for SpawnThreadInStack {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}
impl TryNamedRegistryItemOverride<MaybeStaticFunction, SpawnThreadFuncOverride>
    for SpawnThreadInStack
{
    fn try_override(
        SpawnThreadFuncOverride {
            types,
            static_functions,
            imported_func_count,
            ..
        }: SpawnThreadFuncOverride,
    ) -> HQResult<MaybeStaticFunction> {
        let stack_struct_type = types.register_comp::<TStackStruct, _>()?;
        let dyn_array_push = static_functions.register::<DynArrayPush<StackStructRef>, u32>()?;
        Ok(MaybeStaticFunction {
            static_function: Some(StaticFunction {
                export: None,
                instructions: Box::from(wasm_const![
                    LocalGet(0),
                    Call(
                        imported_func_count
                            + static_functions.register::<DynArrayPop<StackStructRef>, u32>()?
                    ),
                    Drop,
                    LocalGet(0),
                    LocalGet(3),
                    RefNull(HeapType::Abstract {
                        shared: false,
                        ty: wasm_encoder::AbstractHeapType::Struct
                    }),
                    StructNew(stack_struct_type),
                    Call(imported_func_count + dyn_array_push), // TODO: this will do unnecessary bounds checks. Just mutate the last element.
                    LocalGet(0),
                    LocalGet(1),
                    LocalGet(2),
                    StructNew(stack_struct_type),
                    Call(imported_func_count + dyn_array_push),
                ] as &[_]),
                params: Box::from([
                    <TNonNullable<TStackArray>>::ty(&types)?,
                    <TNonNullable<TStepFunc>>::ty(&types)?,
                    StackStructRef::ty(&types)?,
                    <TNonNullable<TStepFunc>>::ty(&types)?,
                ]),
                returns: Box::from([]),
                locals: Box::from([]),
            }),
            maybe_populate: || None,
        })
    }
}

/// Spawn a new thread with the provided step function. This does not call it
/// immediately, instead leaving that for the scheduler or calling function to do so.
///
/// Takes 3 parameters:
/// - i32             - the index of the target to spawn a thread for
/// - step funcref    - the step to spawn
/// - ref null struct - the stack struct to spawn it with
pub struct SpawnNewThread;
impl NamedRegistryItem<MaybeStaticFunction> for SpawnNewThread {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}

impl TryNamedRegistryItemOverride<MaybeStaticFunction, SpawnThreadFuncOverride> for SpawnNewThread {
    fn try_override(
        SpawnThreadFuncOverride {
            types,
            globals,
            static_functions,
            num_sprites,
            imported_func_count,
            imported_global_count,
        }: SpawnThreadFuncOverride,
    ) -> HQResult<MaybeStaticFunction> {
        let stack_struct_type = types.register_comp::<TStackStruct, _>()?;
        let target_threads_type = types.register_comp::<TTargetThreadArray, _>()?;
        let target_threads_global: u32 = globals.threadss(&types, num_sprites)?;
        Ok(MaybeStaticFunction {
            static_function: Some(StaticFunction {
                export: None,
                params: Box::from([
                    ValType::I32,
                    <TNonNullable<TStepFunc>>::ty(&types)?,
                    StackStructRef::ty(&types)?,
                ]),
                returns: Box::from([]),
                locals: Box::from([<TNonNullable<TThreadArray>>::ty(&types)?]),
                instructions: {
                    (wasm_const![
                        LocalGet(0),
                        I32Eqz, // if this is not the stage, we need to find its layer
                        If(WasmBlockType::Empty),
                        LocalGet(0),
                        I32Const(mem_layout::sprite::BLOCK_SIZE as i32),
                        I32Mul,
                        I32Load16U(MemArg {
                            offset: (mem_layout::stage::BLOCK_SIZE + mem_layout::sprite::LAYER)
                                as u64,
                            align: 1,
                            memory_index: 0,
                        }),
                        LocalSet(0), // local 0 is now index of sprite in
                        End,
                        GlobalGet(imported_global_count + target_threads_global),
                        LocalGet(0),
                        ArrayGet(target_threads_type),
                        I32Const(8),
                        Call(
                            imported_func_count
                                + static_functions
                                    .register::<DynArrayNew<StackStructRef>, u32>()?
                        ),
                        LocalTee(3),
                        LocalGet(1),
                        LocalGet(2),
                        StructNew(stack_struct_type),
                        Call(
                            imported_func_count
                                + static_functions
                                    .register::<DynArrayPush<StackStructRef>, u32>()?
                        ),
                        LocalGet(3),
                        Call(
                            imported_func_count
                                + static_functions
                                    .register::<DynArrayPush<TNullable<TStackArray>>, u32>()?
                        ),
                        End,
                    ] as &[_])
                        .into()
                },
            }),
            maybe_populate: || None,
        })
    }
}
