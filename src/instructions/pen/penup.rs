use wasm_encoder::MemArg;

use super::super::prelude::*;
use crate::wasm::{StepTarget, mem_layout};

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        hq_bad_proj!("looks_setpendown called in stage")
    };
    let sprite_offset =
        mem_layout::stage::BLOCK_SIZE + wasm_target_index * mem_layout::sprite::BLOCK_SIZE;
    Ok(wasm![
        I32Const(0),
        I32Const(0),
        I32Store8(MemArg {
            offset: (sprite_offset + mem_layout::sprite::PEN_DOWN).into(),
            align: 0,
            memory_index: 0,
        }),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; pen_penup; ; }
