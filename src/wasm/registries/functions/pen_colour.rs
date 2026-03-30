use mem_layout::{sprite as sprite_layout, stage as stage_layout};
use wasm_encoder::{
    BlockType as WasmBlockType, MemArg, ValType,
};
use wasm_gen::wasm_const;

use super::{MaybeStaticFunction, StaticFunction};
use crate::prelude::*;
use crate::wasm::{f32_to_ieeef32, mem_layout};

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

/// Updates the stored RGBA pen colour from the HSV colour.
///
/// Takes 1 paramter, an i32 corresponding to the target index
///
/// Not overridable.
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
    SAT
}

/// Updates the stored HSV pen colour from the RGBA colour.
///
/// Takes one parameter, an i32 corresponding to the target index.
///
/// Not overridable.
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
