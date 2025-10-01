use super::super::prelude::*;
use crate::wasm::{StepTarget, mem_layout};
use mem_layout::{sprite as sprite_layout, stage as stage_layout};
use wasm_encoder::MemArg;

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        hq_bad_proj!("motion_direction called in stage")
    };
    Ok(wasm![
        I32Const(0),
        F64Load(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::ROTATION)
                .into(),
            align: 3,
            memory_index: 0
        }),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::Singleton(IrType::FloatReal))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; motion_direction; }
