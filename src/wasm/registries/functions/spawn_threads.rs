use wasm_encoder::{AbstractHeapType, HeapType, RefType, ValType};
use wasm_gen::wasm_const;

use super::{MaybeStaticFunction, StaticFunction};
use crate::prelude::*;

/// Spawns a new thread in the same stack (i.e. a thread that yields back to the current
/// thread once it completes.)
///
/// Takes 4 parameters:
/// - i32 - the current thread index
/// - step funcref - the step to spawn
/// - structref - the structref to pass to the step being spawned
/// - step funcref - the step to return to after
///
/// Override with:
/// - u32 - the index of the step func type
/// - u32 - the index of the stack struct type
/// - u32 - the index of the stack array type
/// - u32 - the index of the thread struct type
/// - u32 - the index of the threads table
pub struct SpawnThreadInStack;
impl NamedRegistryItem<MaybeStaticFunction> for SpawnThreadInStack {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}
pub type SpawnThreadInStackOverride = (u32, u32, u32, u32, u32);
impl NamedRegistryItemOverride<MaybeStaticFunction, SpawnThreadInStackOverride>
    for SpawnThreadInStack
{
    fn r#override(
        (func_ty, stack_struct_type, stack_array_type, thread_struct_type, threads_table): SpawnThreadInStackOverride,
    ) -> MaybeStaticFunction {
        MaybeStaticFunction {
            static_function: Some(StaticFunction {
                export: None,
                instructions: Box::from(wasm_const![
                    LocalGet(1),
                    LocalGet(2),
                    StructNew(stack_struct_type),
                    LocalSet(4),
                    LocalGet(0),
                    TableGet(threads_table),
                    RefAsNonNull,
                    LocalTee(5),
                    StructGet {
                        struct_type_index: thread_struct_type,
                        field_index: 1,
                    },
                    LocalGet(5),
                    StructGet {
                        struct_type_index: thread_struct_type,
                        field_index: 0,
                    },
                    LocalGet(4),
                    // todo: consider the case where we need to resize the array
                    ArraySet(stack_array_type),
                    LocalGet(5),
                    StructGet {
                        struct_type_index: thread_struct_type,
                        field_index: 1,
                    },
                    LocalGet(5),
                    StructGet {
                        struct_type_index: thread_struct_type,
                        field_index: 0,
                    },
                    I32Const(1),
                    I32Sub,
                    ArrayGet(stack_array_type),
                    LocalGet(3),
                    StructSet {
                        struct_type_index: stack_struct_type,
                        field_index: 0,
                    },
                    LocalGet(5),
                    LocalGet(5),
                    StructGet {
                        struct_type_index: thread_struct_type,
                        field_index: 0,
                    },
                    I32Const(1),
                    I32Add,
                    StructSet {
                        struct_type_index: thread_struct_type,
                        field_index: 0,
                    },
                    End
                ] as &[_]),
                params: Box::from([
                    ValType::I32,
                    ValType::Ref(RefType {
                        nullable: false,
                        heap_type: HeapType::Concrete(func_ty),
                    }),
                    ValType::Ref(RefType {
                        nullable: true,
                        heap_type: wasm_encoder::HeapType::Abstract {
                            shared: false,
                            ty: AbstractHeapType::Struct,
                        },
                    }),
                    ValType::Ref(RefType {
                        nullable: false,
                        heap_type: HeapType::Concrete(func_ty),
                    }),
                ]),
                returns: Box::from([]),
                locals: Box::from([
                    ValType::Ref(RefType {
                        nullable: false,
                        heap_type: HeapType::Concrete(stack_struct_type),
                    }),
                    ValType::Ref(RefType {
                        nullable: false,
                        heap_type: HeapType::Concrete(thread_struct_type),
                    }),
                ]),
            }),
            maybe_populate: || None,
        }
    }
}

/// Spawn a new thread with the provided step function. This does not call it
/// immediately, instead leaving that for the scheduler or calling function to do so.
///
/// Takes 3 parameters:
/// - step funcref    - the step to spawn
/// - ref null struct - the stack struct to spawn it with
/// - i32             - the index of the sprite to spawn a thread for, or -1 for the stage
///
/// Override with:
/// - u32 - the index of the step func type
/// - u32 - the index of the stack struct type
/// - u32 - the index of the stack array type
/// - u32 - the index of the thread struct type
/// - u32 - the index of the threads array
pub struct SpawnNewThread;
impl NamedRegistryItem<MaybeStaticFunction> for SpawnNewThread {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}
pub struct SpawnNewThreadOverride {
    func_ty: u32,
    stack_struct_ty: u32,
    stack_array_ty: u32,
    thread_struct_ty: u32,
    threads_array_index: u32,
    threads_array_ty: u32,
}

impl NamedRegistryItemOverride<MaybeStaticFunction, SpawnNewThreadOverride> for SpawnNewThread {
    fn r#override(
        SpawnNewThreadOverride {
            func_ty,
            stack_struct_ty,
            stack_array_ty,
            thread_struct_ty,
            threads_array_index,
            threads_array_ty,
        }: SpawnNewThreadOverride,
    ) -> MaybeStaticFunction {
        MaybeStaticFunction {
            static_function: Some(StaticFunction {
                export: None,
                params: Box::from([
                    ValType::Ref(RefType {
                        nullable: false,
                        heap_type: HeapType::Concrete(func_ty),
                    }),
                    ValType::Ref(RefType {
                        nullable: true,
                        heap_type: wasm_encoder::HeapType::Abstract {
                            shared: false,
                            ty: AbstractHeapType::Struct,
                        },
                    }),
                ]),
                returns: Box::from([]),
                locals: Box::from([ValType::Ref(RefType {
                    nullable: false,
                    heap_type: HeapType::Concrete(stack_array_ty),
                })]),
                instructions: {
                    const STACK_ARRAY_LOCAL: u32 = 3;
                    (wasm_const![
                        // TODO get
                        I32Const(1), // stack size
                        // todo: play around with initial size of stack array
                        I32Const(8),
                        ArrayNewDefault(stack_array_ty),
                        LocalTee(STACK_ARRAY_LOCAL),
                        StructNew(thread_struct_ty),
                        I32Const(0),                // index 0 into stack array
                        I32Const(1),                // stack size
                        LocalGet(0),                // step func
                        LocalGet(1),                // stack struct param
                        StructNew(stack_struct_ty), // stack struct
                        ArraySet(stack_array_ty), // set 0th element of stack array to stack struct
                        ArraySet(threads_array_ty),
                        End,
                    ] as &[_])
                        .into()
                },
            }),
            maybe_populate: || None,
        }
    }
}
