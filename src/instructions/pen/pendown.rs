use wasm_encoder::MemArg;

use super::super::prelude::*;
use crate::wasm::{StepTarget, mem_layout};

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let func_index = func.registries().external_functions().register(
        ("pen", "point".into()),
        (vec![ValType::F64, ValType::F64, ValType::F64, ValType::F32, ValType::F32, ValType::F32, ValType::F32], vec![]),
    )?;
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        hq_bad_proj!("looks_setpendown called in stage")
    };
    let sprite_offset = mem_layout::stage::BLOCK_SIZE + wasm_target_index * mem_layout::sprite::BLOCK_SIZE;
    let local_index = func.local(ValType::I32)?;
    Ok(wasm![
        I32Const(0),
        I32Const(1),
        I32Store8(MemArg {
            offset: (sprite_offset + mem_layout::sprite::PEN_DOWN).into(),
            align: 0,
            memory_index: 0,
        }),
        I32Const(0),
        F64Load(MemArg {
            offset: (sprite_offset + mem_layout::sprite::PEN_SIZE).into(),
            align: 3,
            memory_index: 0,
        }),
        I32Const(0),
        F64Load(MemArg {
            offset: (sprite_offset + mem_layout::sprite::X).into(),
            align: 3,
            memory_index: 0,
        }),
        I32Const(0),
        F64Load(MemArg {
            offset: (sprite_offset + mem_layout::sprite::Y).into(),
            align: 3,
            memory_index: 0,
        }),
        I32Const(0),
        F32Load(MemArg {
            offset: (sprite_offset + mem_layout::sprite::PEN_COLOR_R).into(),
            align: 2,
            memory_index: 0,
        }),
        I32Const(0),
        F32Load(MemArg {
            offset: (sprite_offset + mem_layout::sprite::PEN_COLOR_G).into(),
            align: 2,
            memory_index: 0,
        }),
        I32Const(0),
        F32Load(MemArg {
            offset: (sprite_offset + mem_layout::sprite::PEN_COLOR_B).into(),
            align: 2,
            memory_index: 0,
        }),
        I32Const(0),
        F32Load(MemArg {
            offset: (sprite_offset + mem_layout::sprite::PEN_COLOR_A).into(),
            align: 2,
            memory_index: 0,
        }),
        Call(func_index),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; pen_pendown; ; }
