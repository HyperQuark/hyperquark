use wasm_encoder::MemArg;

use super::super::prelude::*;
use crate::wasm::{mem_layout, StepTarget};

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        0
    };
    let offset = mem_layout::stage::BLOCK_SIZE
        + wasm_target_index * mem_layout::sprite::BLOCK_SIZE
        + mem_layout::sprite::COSTUME;

    Ok(wasm![
        I32Const(0),
        I32Store(MemArg {
            offset: offset.into(),
            align: 2,
            memory_index: 0,
        }),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::Singleton(IrType::IntPos))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; looks_costumenumber; ; }
