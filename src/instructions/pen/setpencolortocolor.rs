use super::super::prelude::*;
use crate::wasm::{StepTarget, mem_layout, registries};
use registries::functions::static_functions::UpdatePenColorFromRGB;
use wasm_encoder::{BlockType, MemArg};

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let t1 = inputs[0];
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        hq_bad_proj!("looks_setsizeto called in stage")
    };
    let mem_pos =
        mem_layout::stage::BLOCK_SIZE + wasm_target_index * mem_layout::sprite::BLOCK_SIZE;
    let local_index = func.local(ValType::I32)?;
    let rgb2hsv_func = func
        .registries()
        .static_functions()
        .register::<UpdatePenColorFromRGB, _>()?;
    Ok(wasm![LocalSet(local_index), I32Const(0),]
        .into_iter()
        .chain(match t1 {
            IrType::ColorARGB => {
                let temp_local = func.local(ValType::I32)?;
                wasm![
                    LocalGet(local_index),
                    I32Const(24),
                    I32ShrS,
                    I32Const(0xFF),
                    I32And,
                    LocalTee(temp_local),
                    If(BlockType::Result(ValType::F32)),
                    LocalGet(temp_local),
                    F32ConvertI32S,
                    F32Const(255.0.into()),
                    F32Div,
                    Else,
                    // scratch doesn't allow totally transparent alpha values for rgb colours - see
                    // https://github.com/scratchfoundation/scratch-vm/blob/b3266a0cfe5122f20b72ccd738a3dd4dff4fc5a5/src/util/color.js#L50
                    F32Const(1.0.into()),
                    End,
                ]
            }
            IrType::ColorRGB => wasm![F32Const(1.0.into())],
            _ => hq_bug!("bad input type to pen_setPenColorToColor"),
        })
        .chain(wasm![

            F32Store(MemArg {
                offset: (mem_pos + mem_layout::sprite::PEN_COLOR_A).into(),
                align: 2,
                memory_index: 0
            }),
            I32Const(0),
            LocalGet(local_index),
            I32Const(16),
            I32ShrS,
            I32Const(0xFF),
            I32And,
            F32ConvertI32S,
            F32Const(255.0.into()),
            F32Div,
            F32Store(MemArg {
                offset: (mem_pos + mem_layout::sprite::PEN_COLOR_R).into(),
                align: 2,
                memory_index: 0
            }),
            I32Const(0),
            LocalGet(local_index),
            I32Const(8),
            I32ShrS,
            I32Const(0xFF),
            I32And,
            F32ConvertI32S,
            F32Const(255.0.into()),
            F32Div,
            F32Store(MemArg {
                offset: (mem_pos + mem_layout::sprite::PEN_COLOR_G).into(),
                align: 2,
                memory_index: 0
            }),
            I32Const(0),
            LocalGet(local_index),
            I32Const(0xFF),
            I32And,
            F32ConvertI32S,
            F32Const(255.0.into()),
            F32Div,
            F32Store(MemArg {
                offset: (mem_pos + mem_layout::sprite::PEN_COLOR_B).into(),
                align: 2,
                memory_index: 0
            }),
            I32Const(
                wasm_target_index
                    .try_into()
                    .map_err(|_| make_hq_bug!("target index out of bounds"))?
            ),
            #StaticFunctionCall(rgb2hsv_func),
        ])
        .collect())
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Color]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; pen_setpencolortocolor; t ; }
