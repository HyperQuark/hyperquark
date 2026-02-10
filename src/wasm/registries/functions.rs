#![allow(clippy::cast_possible_wrap, reason = "can't use try_into in const")]

use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection, ImportSection,
    Instruction as WInstruction, ValType,
};

use super::TypeRegistry;
use crate::prelude::*;
use crate::registry::{MapRegistry, Registry};

pub type ExternalFunctionRegistry =
    MapRegistry<(&'static str, Box<str>), (Vec<ValType>, Vec<ValType>)>;

impl ExternalFunctionRegistry {
    pub fn finish(self, imports: &mut ImportSection, type_registry: &TypeRegistry) -> HQResult<()> {
        for ((module, name), (params, results)) in self.registry().take() {
            let type_index = type_registry.function(params, results)?;
            imports.import(module, &name, EntityType::Function(type_index));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct StaticFunction {
    pub instructions: Box<[WInstruction<'static>]>,
    pub params: Box<[ValType]>,
    pub returns: Box<[ValType]>,
    pub locals: Box<[ValType]>,
    pub export: Option<Box<str>>,
}

#[derive(Clone)]
pub struct MaybeStaticFunction {
    pub static_function: Option<StaticFunction>,
    pub maybe_populate: fn() -> Option<StaticFunction>,
}

pub struct StaticFunctionRegistrar;
impl RegistryType for StaticFunctionRegistrar {
    type Value = MaybeStaticFunction;
}
pub type StaticFunctionRegistry = NamedRegistry<StaticFunctionRegistrar>;

impl StaticFunctionRegistry {
    pub fn finish(
        self,
        functions: &mut FunctionSection,
        exports: &mut ExportSection,
        codes: &mut CodeSection,
        type_registry: &TypeRegistry,
        imported_func_count: u32,
    ) -> HQResult<()> {
        for (
            _name,
            MaybeStaticFunction {
                static_function,
                maybe_populate,
            },
        ) in self.registry().take()
        {
            let Some(StaticFunction {
                instructions,
                params,
                returns,
                locals,
                export,
            }) = static_function.map_or_else(maybe_populate, Some)
            else {
                hq_bug!(
                    "static functions must either be overriden, or have a non-None maybe_populate \
                     field"
                )
            };
            let type_index = type_registry.function(params.into(), returns.into())?;
            functions.function(type_index);
            let mut func = Function::new_with_locals_types(locals.iter().copied());
            for instruction in instructions {
                func.instruction(&instruction);
            }
            codes.function(&func);
            if let Some(export_name) = export {
                exports.export(
                    &export_name,
                    ExportKind::Func,
                    imported_func_count + functions.len() - 1,
                );
            }
        }
        Ok(())
    }
}

pub mod static_functions {
    use mem_layout::{sprite as sprite_layout, stage as stage_layout};
    use wasm_encoder::{
        AbstractHeapType, BlockType as WasmBlockType, HeapType, MemArg, RefType, ValType,
    };
    use wasm_gen::wasm_const;

    use super::{MaybeStaticFunction, StaticFunction};
    use crate::prelude::*;
    use crate::wasm::{f32_to_ieeef32, mem_layout};

    /// Mark a waiting flag as done.
    ///
    /// This is designed to be exported (as `"mark_waiting_flag"`) and called by JS.
    ///
    /// Takes 1 parameter:
    /// - A nonnull struct with a single i8 field.
    pub struct MarkWaitingFlag;
    impl NamedRegistryItem<MaybeStaticFunction> for MarkWaitingFlag {
        const VALUE: MaybeStaticFunction = MaybeStaticFunction {
            static_function: None,
            maybe_populate: || None,
        };
    }
    pub type MarkWaitingFlagOverride = u32;
    impl NamedRegistryItemOverride<MaybeStaticFunction, MarkWaitingFlagOverride> for MarkWaitingFlag {
        fn r#override(i8_struct_ty: u32) -> MaybeStaticFunction {
            MaybeStaticFunction {
                static_function: Some(StaticFunction {
                    export: Some("mark_waiting_flag".into()),
                    instructions: Box::from(wasm_const![
                        LocalGet(0),
                        I32Const(1),
                        StructSet {
                            struct_type_index: i8_struct_ty,
                            field_index: 0
                        },
                        End,
                    ] as &[_]),
                    params: Box::new([ValType::Ref(RefType {
                        nullable: false,
                        heap_type: HeapType::Concrete(i8_struct_ty),
                    })]),
                    returns: Box::new([]),
                    locals: Box::new([]),
                }),
                maybe_populate: || None,
            }
        }
    }

    /// Spawns a new thread in the same stack (i.e. a thread that yields back to the current
    /// thread once it completes.)
    ///
    /// Takes 4 parameters:
    /// - i32 - the current thread index
    /// - step funcref - the step to spawn
    /// - structref - the structref to pass to the step being spawned
    /// - step funcref - the step to return to after
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

    pub struct SpawnNewThread;
    impl NamedRegistryItem<MaybeStaticFunction> for SpawnNewThread {
        const VALUE: MaybeStaticFunction = MaybeStaticFunction {
            static_function: None,
            maybe_populate: || None,
        };
    }
    pub type SpawnNewThreadOverride = (u32, u32, u32, u32, u32);
    impl NamedRegistryItemOverride<MaybeStaticFunction, SpawnNewThreadOverride> for SpawnNewThread {
        fn r#override(
            (func_ty, stack_struct_ty, stack_array_ty, thread_struct_ty, threads_table_index): SpawnNewThreadOverride,
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
                    locals: Box::from([]),
                    instructions: (wasm_const![
                        I32Const(1),
                        LocalGet(0),
                        LocalGet(1),
                        StructNew(stack_struct_ty),
                        // todo: play around with initial size of stack array
                        RefNull(HeapType::Concrete(stack_struct_ty)),
                        RefNull(HeapType::Concrete(stack_struct_ty)),
                        RefNull(HeapType::Concrete(stack_struct_ty)),
                        RefNull(HeapType::Concrete(stack_struct_ty)),
                        RefNull(HeapType::Concrete(stack_struct_ty)),
                        RefNull(HeapType::Concrete(stack_struct_ty)),
                        RefNull(HeapType::Concrete(stack_struct_ty)),
                        ArrayNewFixed {
                            array_size: 8,
                            array_type_index: stack_array_ty,
                        },
                        StructNew(thread_struct_ty),
                        I32Const(1),
                        TableGrow(threads_table_index),
                        Drop,
                        End,
                    ] as &[_])
                        .into(),
                }),
                maybe_populate: || None,
            }
        }
    }

    index_counter! {
        hsv2rgb_locals
        SPRITE_INDEX
        MEM_POS
        HUE SAT VAL
        REGION
        REMAINDER
        P Q T
        R G B
        VAL_F
    }

    pub struct UpdatePenColorFromHSV;
    impl NamedRegistryItem<MaybeStaticFunction> for UpdatePenColorFromHSV {
        const VALUE: MaybeStaticFunction = MaybeStaticFunction {
            static_function: None,
            maybe_populate: || {
                Some(StaticFunction {
                    export: None,
                    params: Box::from([ValType::I32]),
                    returns: Box::from([]),
                    locals: Box::from({
                        const PARAMS_NUM: usize = 1;
                        let mut locals =
                            [ValType::I32; hsv2rgb_locals::BLOCK_SIZE as usize - PARAMS_NUM];
                        locals[hsv2rgb_locals::VAL_F as usize - PARAMS_NUM] = ValType::F32;
                        locals
                    }),
                    instructions: (wasm_const![
                        // hsv->rgb based off of https://stackoverflow.com/a/14733008
                        LocalGet(hsv2rgb_locals::SPRITE_INDEX),
                        I32Const(sprite_layout::BLOCK_SIZE as i32),
                        I32Mul,
                        I32Const(stage_layout::BLOCK_SIZE as i32),
                        I32Add,
                        LocalTee(hsv2rgb_locals::MEM_POS), // position in memory of sprite info
                        F32Load(MemArg {
                            offset: sprite_layout::PEN_COLOR.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        F32Const(f32_to_ieeef32(2.55)),
                        F32Mul,
                        I32TruncF32S,
                        LocalSet(hsv2rgb_locals::HUE),
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        F32Load(MemArg {
                            offset: sprite_layout::PEN_SATURATION.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        F32Const(f32_to_ieeef32(2.55)),
                        F32Mul,
                        I32TruncF32S,
                        LocalSet(hsv2rgb_locals::SAT), // saturation ∈ [0, 256)
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        F32Load(MemArg {
                            offset: sprite_layout::PEN_BRIGHTNESS.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        F32Const(f32_to_ieeef32(2.55)),
                        F32Mul,
                        I32TruncF32S,
                        LocalSet(hsv2rgb_locals::VAL), // value ∈ [0, 256)
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        F32Const(f32_to_ieeef32(100.0)),
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        F32Load(MemArg {
                            offset: sprite_layout::PEN_TRANSPARENCY.into(),
                            align: 2,
                            memory_index: 0,
                        }), // transparency ∈ [0, 100]
                        F32Sub,
                        F32Const(f32_to_ieeef32(100.0)),
                        F32Div, // alpha ∈ [0, 1]
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_COLOR_A.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        F32Load(MemArg {
                            offset: sprite_layout::PEN_COLOR_A.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        F32Const(f32_to_ieeef32(0.01)),
                        F32Lt,
                        If(WasmBlockType::Empty),
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        F32Const(f32_to_ieeef32(0.0)),
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_COLOR_A.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        Return, // if alpha is 0, return (it is already set to 0 so it doesn't matter what r, g & b are)
                        End,
                        LocalGet(hsv2rgb_locals::SAT),
                        I32Eqz,
                        If(WasmBlockType::Empty),
                        LocalGet(hsv2rgb_locals::VAL),
                        F32ConvertI32S,
                        F32Const(f32_to_ieeef32(255.0)),
                        F32Div,
                        LocalSet(hsv2rgb_locals::VAL_F),
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        LocalGet(hsv2rgb_locals::VAL_F),
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_COLOR_R.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        LocalGet(hsv2rgb_locals::VAL_F),
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_COLOR_G.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        LocalGet(hsv2rgb_locals::VAL_F),
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_COLOR_B.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        Return,
                        End,
                        LocalGet(hsv2rgb_locals::HUE),
                        I32Const(43),
                        I32DivU,
                        LocalSet(hsv2rgb_locals::REGION), // 'region'
                        LocalGet(hsv2rgb_locals::HUE),
                        I32Const(43),
                        I32RemU,
                        I32Const(6),
                        I32Mul,
                        LocalSet(hsv2rgb_locals::REMAINDER), // 'remainder'
                        I32Const(255),
                        LocalGet(hsv2rgb_locals::SAT),
                        I32Sub,
                        LocalGet(hsv2rgb_locals::VAL),
                        I32Mul,
                        I32Const(8),
                        I32ShrU,
                        LocalSet(hsv2rgb_locals::P), // 'p'
                        I32Const(255),
                        LocalGet(hsv2rgb_locals::REMAINDER),
                        LocalGet(hsv2rgb_locals::SAT),
                        I32Mul,
                        I32Const(8),
                        I32ShrU,
                        I32Sub,
                        LocalGet(hsv2rgb_locals::VAL),
                        I32Mul,
                        I32Const(8),
                        I32ShrU,
                        LocalSet(hsv2rgb_locals::Q), // 'q'
                        I32Const(255),
                        I32Const(255),
                        LocalGet(hsv2rgb_locals::REMAINDER),
                        I32Sub,
                        LocalGet(hsv2rgb_locals::SAT),
                        I32Mul,
                        I32Const(8),
                        I32ShrU,
                        I32Sub,
                        LocalGet(hsv2rgb_locals::VAL),
                        I32Mul,
                        I32Const(8),
                        I32ShrU,
                        LocalSet(hsv2rgb_locals::T), // 't'
                        LocalGet(hsv2rgb_locals::REGION),
                        I32Eqz,
                        If(WasmBlockType::Empty),
                        LocalGet(hsv2rgb_locals::VAL),
                        LocalSet(hsv2rgb_locals::R),
                        LocalGet(hsv2rgb_locals::T),
                        LocalSet(hsv2rgb_locals::G),
                        LocalGet(hsv2rgb_locals::P),
                        LocalSet(hsv2rgb_locals::B),
                        End,
                        LocalGet(hsv2rgb_locals::REGION),
                        I32Const(1),
                        I32Eq,
                        If(WasmBlockType::Empty),
                        LocalGet(hsv2rgb_locals::Q),
                        LocalSet(hsv2rgb_locals::R),
                        LocalGet(hsv2rgb_locals::VAL),
                        LocalSet(hsv2rgb_locals::G),
                        LocalGet(hsv2rgb_locals::P),
                        LocalSet(hsv2rgb_locals::B),
                        End,
                        LocalGet(hsv2rgb_locals::REGION),
                        I32Const(2),
                        I32Eq,
                        If(WasmBlockType::Empty),
                        LocalGet(hsv2rgb_locals::P),
                        LocalSet(hsv2rgb_locals::R),
                        LocalGet(hsv2rgb_locals::VAL),
                        LocalSet(hsv2rgb_locals::G),
                        LocalGet(hsv2rgb_locals::T),
                        LocalSet(hsv2rgb_locals::B),
                        End,
                        LocalGet(hsv2rgb_locals::REGION),
                        I32Const(3),
                        I32Eq,
                        If(WasmBlockType::Empty),
                        LocalGet(hsv2rgb_locals::P),
                        LocalSet(hsv2rgb_locals::R),
                        LocalGet(hsv2rgb_locals::Q),
                        LocalSet(hsv2rgb_locals::G),
                        LocalGet(hsv2rgb_locals::VAL),
                        LocalSet(hsv2rgb_locals::B),
                        End,
                        LocalGet(hsv2rgb_locals::REGION),
                        I32Const(4),
                        I32Eq,
                        If(WasmBlockType::Empty),
                        LocalGet(hsv2rgb_locals::T),
                        LocalSet(hsv2rgb_locals::R),
                        LocalGet(hsv2rgb_locals::P),
                        LocalSet(hsv2rgb_locals::G),
                        LocalGet(hsv2rgb_locals::VAL),
                        LocalSet(hsv2rgb_locals::B),
                        End,
                        LocalGet(hsv2rgb_locals::REGION),
                        I32Const(5),
                        I32Eq,
                        If(WasmBlockType::Empty),
                        LocalGet(hsv2rgb_locals::VAL),
                        LocalSet(hsv2rgb_locals::R),
                        LocalGet(hsv2rgb_locals::P),
                        LocalSet(hsv2rgb_locals::G),
                        LocalGet(hsv2rgb_locals::Q),
                        LocalSet(hsv2rgb_locals::B),
                        End,
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        LocalGet(hsv2rgb_locals::R),
                        F32ConvertI32S,
                        F32Const(f32_to_ieeef32(255.0)),
                        F32Div,
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_COLOR_R.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        LocalGet(hsv2rgb_locals::G),
                        F32ConvertI32S,
                        F32Const(f32_to_ieeef32(255.0)),
                        F32Div,
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_COLOR_G.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        LocalGet(hsv2rgb_locals::MEM_POS),
                        LocalGet(hsv2rgb_locals::B),
                        F32ConvertI32S,
                        F32Const(f32_to_ieeef32(255.0)),
                        F32Div,
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_COLOR_B.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        End,
                    ] as &[_])
                        .into(),
                })
            },
        };
    }

    index_counter! {
        rgb2hsv_locals
        SPRITE_INDEX
        MEM_POS
        R G B A
        RGB_MIN RGB_MAX
        HUE SAT
    }

    pub struct UpdatePenColorFromRGB;
    impl NamedRegistryItem<MaybeStaticFunction> for UpdatePenColorFromRGB {
        const VALUE: MaybeStaticFunction = MaybeStaticFunction {
            static_function: None,
            maybe_populate: || {
                Some(StaticFunction {
                    export: None,
                    params: Box::from([ValType::I32]),
                    returns: Box::from([]),
                    locals: Box::from({
                        const PARAMS_NUM: usize = 1;
                        let mut locals =
                            [ValType::I32; rgb2hsv_locals::BLOCK_SIZE as usize - PARAMS_NUM];
                        locals[rgb2hsv_locals::A as usize - PARAMS_NUM] = ValType::F32;
                        locals
                    }),
                    instructions: (wasm_const![
                        // rgb->hsv based off of https://stackoverflow.com/a/14733008
                        LocalGet(rgb2hsv_locals::SPRITE_INDEX),
                        I32Const(sprite_layout::BLOCK_SIZE as i32),
                        I32Mul,
                        I32Const(stage_layout::BLOCK_SIZE as i32),
                        I32Add,
                        LocalTee(rgb2hsv_locals::MEM_POS), // position in memory of sprite info
                        F32Load(MemArg {
                            offset: sprite_layout::PEN_COLOR_R.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        F32Const(f32_to_ieeef32(255.0)),
                        F32Mul,
                        I32TruncF32S,
                        LocalSet(rgb2hsv_locals::R),
                        LocalGet(rgb2hsv_locals::MEM_POS),
                        F32Load(MemArg {
                            offset: sprite_layout::PEN_COLOR_G.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        F32Const(f32_to_ieeef32(255.0)),
                        F32Mul,
                        I32TruncF32S,
                        LocalSet(rgb2hsv_locals::G),
                        LocalGet(rgb2hsv_locals::MEM_POS),
                        F32Load(MemArg {
                            offset: sprite_layout::PEN_COLOR_B.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        F32Const(f32_to_ieeef32(255.0)),
                        F32Mul,
                        I32TruncF32S,
                        LocalSet(rgb2hsv_locals::B),
                        LocalGet(rgb2hsv_locals::MEM_POS),
                        LocalGet(rgb2hsv_locals::MEM_POS),
                        F32Load(MemArg {
                            offset: sprite_layout::PEN_COLOR_A.into(),
                            align: 2,
                            memory_index: 0,
                        }), // transparency ∈ [0, 100]
                        F32Const(f32_to_ieeef32(100.0)),
                        F32Mul, // alpha ∈ [0, 1]
                        LocalTee(rgb2hsv_locals::A),
                        F32Const(f32_to_ieeef32(100.0)),
                        F32Sub,
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_TRANSPARENCY.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        // we don't need to check for alpha=0 to shortcircuit, because scratch doesn't allow
                        // alpha=0 for rgb colours
                        LocalGet(rgb2hsv_locals::R),
                        LocalGet(rgb2hsv_locals::G),
                        LocalGet(rgb2hsv_locals::R),
                        LocalGet(rgb2hsv_locals::G),
                        I32LtS,
                        Select,
                        LocalTee(rgb2hsv_locals::RGB_MIN),
                        LocalGet(rgb2hsv_locals::B),
                        LocalGet(rgb2hsv_locals::RGB_MIN),
                        LocalGet(rgb2hsv_locals::B),
                        I32LtS,
                        Select,
                        LocalSet(rgb2hsv_locals::RGB_MIN),
                        LocalGet(rgb2hsv_locals::R),
                        LocalGet(rgb2hsv_locals::G),
                        LocalGet(rgb2hsv_locals::R),
                        LocalGet(rgb2hsv_locals::G),
                        I32GtS,
                        Select,
                        LocalTee(rgb2hsv_locals::RGB_MAX),
                        LocalGet(rgb2hsv_locals::B),
                        LocalGet(rgb2hsv_locals::RGB_MAX),
                        LocalGet(rgb2hsv_locals::B),
                        I32GtS,
                        Select,
                        LocalSet(rgb2hsv_locals::RGB_MAX),
                        LocalGet(rgb2hsv_locals::MEM_POS),
                        LocalGet(rgb2hsv_locals::RGB_MAX),
                        F32ConvertI32S,
                        F32Const(f32_to_ieeef32(2.55)),
                        F32Div,
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_BRIGHTNESS.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        LocalGet(rgb2hsv_locals::RGB_MAX),
                        I32Eqz,
                        If(WasmBlockType::Empty),
                        LocalGet(rgb2hsv_locals::MEM_POS),
                        F32Const(f32_to_ieeef32(0.0)),
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_COLOR.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        LocalGet(rgb2hsv_locals::MEM_POS),
                        F32Const(f32_to_ieeef32(0.0)),
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_SATURATION.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        Return,
                        End,
                        LocalGet(rgb2hsv_locals::RGB_MAX),
                        LocalGet(rgb2hsv_locals::RGB_MIN),
                        I32Sub,
                        I32Const(255),
                        I32Mul,
                        LocalGet(rgb2hsv_locals::RGB_MAX),
                        I32DivS,
                        LocalTee(rgb2hsv_locals::SAT),
                        I32Eqz,
                        If(WasmBlockType::Empty),
                        LocalGet(rgb2hsv_locals::MEM_POS),
                        F32Const(f32_to_ieeef32(0.0)),
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_COLOR.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        LocalGet(rgb2hsv_locals::MEM_POS),
                        F32Const(f32_to_ieeef32(0.0)),
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_SATURATION.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        Return,
                        End,
                        LocalGet(rgb2hsv_locals::MEM_POS),
                        LocalGet(rgb2hsv_locals::SAT),
                        F32ConvertI32S,
                        F32Const(f32_to_ieeef32(2.55)),
                        F32Div,
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_SATURATION.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        LocalGet(rgb2hsv_locals::MEM_POS),
                        LocalGet(rgb2hsv_locals::RGB_MAX),
                        LocalGet(rgb2hsv_locals::R),
                        I32Eq,
                        If(WasmBlockType::Result(ValType::I32)),
                        LocalGet(rgb2hsv_locals::G),
                        LocalGet(rgb2hsv_locals::B),
                        I32Sub,
                        LocalGet(rgb2hsv_locals::RGB_MAX),
                        LocalGet(rgb2hsv_locals::RGB_MIN),
                        I32Sub,
                        I32DivS,
                        I32Const(43),
                        I32Mul,
                        Else,
                        LocalGet(rgb2hsv_locals::RGB_MAX),
                        LocalGet(rgb2hsv_locals::G),
                        I32Eq,
                        If(WasmBlockType::Result(ValType::I32)),
                        LocalGet(rgb2hsv_locals::B),
                        LocalGet(rgb2hsv_locals::R),
                        I32Sub,
                        LocalGet(rgb2hsv_locals::RGB_MAX),
                        LocalGet(rgb2hsv_locals::RGB_MIN),
                        I32Sub,
                        I32DivS,
                        I32Const(43),
                        I32Mul,
                        I32Const(85),
                        I32Add,
                        Else,
                        LocalGet(rgb2hsv_locals::R),
                        LocalGet(rgb2hsv_locals::G),
                        I32Sub,
                        LocalGet(rgb2hsv_locals::RGB_MAX),
                        LocalGet(rgb2hsv_locals::RGB_MIN),
                        I32Sub,
                        I32DivS,
                        I32Const(43),
                        I32Mul,
                        I32Const(171),
                        I32Add,
                        End,
                        End,
                        F32ConvertI32S,
                        F32Const(f32_to_ieeef32(2.55)),
                        F32Div,
                        F32Store(MemArg {
                            offset: sprite_layout::PEN_COLOR.into(),
                            align: 2,
                            memory_index: 0,
                        }),
                        End,
                    ] as &[_])
                        .into(),
                })
            },
        };
    }
}
