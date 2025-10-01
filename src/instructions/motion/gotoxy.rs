use super::super::prelude::*;
use crate::wasm::{StepTarget, mem_layout};
use mem_layout::{sprite as sprite_layout, stage as stage_layout};
use wasm_encoder::MemArg;

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let ir_target_index: i32 = func
        .target_index()
        .try_into()
        .map_err(|_| make_hq_bug!("target index out of bounds"))?;
    let move_func_index = func.registries().external_functions().register(
        ("motion", "gotoxy".into()),
        (vec![ValType::F64, ValType::F64, ValType::I32], vec![]),
    )?;
    let pen_func_index = func.registries().external_functions().register(
        ("pen", "line".into()),
        (
            vec![
                ValType::F64,
                ValType::F64,
                ValType::F64,
                ValType::F64,
                ValType::F64,
                ValType::F32,
                ValType::F32,
                ValType::F32,
                ValType::F32,
            ],
            vec![],
        ),
    )?;
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        hq_bad_proj!("motion_gotoxy called in stage")
    };
    let x_local = func.local(ValType::F64)?;
    let y_local = func.local(ValType::F64)?;
    Ok(wasm![
        LocalSet(y_local),
        LocalSet(x_local),
        I32Const(0),
        I32Load8U(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::PEN_DOWN)
                .into(),
            align: 0,
            memory_index: 0
        }),
        If(wasm_encoder::BlockType::Empty),
        I32Const(0),
        F64Load(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::PEN_SIZE)
                .into(),
            align: 3,
            memory_index: 0
        }),
        I32Const(0),
        F64Load(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::X)
                .into(),
            align: 3,
            memory_index: 0
        }),
        I32Const(0),
        F64Load(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::Y)
                .into(),
            align: 3,
            memory_index: 0
        }),
        LocalGet(x_local),
        LocalGet(y_local),
        I32Const(0),
        F32Load(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::PEN_COLOR_R)
                .into(),
            align: 2,
            memory_index: 0
        }),
        I32Const(0),
        F32Load(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::PEN_COLOR_G)
                .into(),
            align: 2,
            memory_index: 0
        }),
        I32Const(0),
        F32Load(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::PEN_COLOR_B)
                .into(),
            align: 2,
            memory_index: 0
        }),
        I32Const(0),
        F32Load(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::PEN_COLOR_A)
                .into(),
            align: 2,
            memory_index: 0
        }),
        Call(pen_func_index),
        End,
        I32Const(0),
        LocalGet(x_local),
        F64Store(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::X)
                .into(),
            align: 3,
            memory_index: 0,
        }),
        I32Const(0),
        LocalGet(y_local),
        F64Store(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::Y)
                .into(),
            align: 3,
            memory_index: 0,
        }),
        LocalGet(x_local),
        LocalGet(y_local),
        I32Const(ir_target_index),
        Call(move_func_index),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Float, IrType::Float]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = true;

crate::instructions_test! {tests; motion_gotoxy; t1, t2 ; }
