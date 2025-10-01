use super::super::prelude::*;
use crate::wasm::{StepTarget, mem_layout};
use mem_layout::{sprite as sprite_layout, stage as stage_layout};
use wasm_encoder::MemArg;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        hq_bad_proj!("motion_pointindirection called in stage")
    };
    let ir_target_index: i32 = func
        .target_index()
        .try_into()
        .map_err(|_| make_hq_bug!("target index out of bounds"))?;
    let local_idx = func.local(ValType::F64)?;
    let t1 = inputs[0];
    let imported_func = func.registries().external_functions().register(
        ("motion", "pointindirection".into()),
        (vec![ValType::I32, ValType::F64], vec![]),
    )?;
    Ok(wasm![
        @nanreduce(t1),
        LocalSet(local_idx),
        I32Const(0),
        LocalGet(local_idx),
        F64Store(MemArg {
            offset: (stage_layout::BLOCK_SIZE
                + wasm_target_index * sprite_layout::BLOCK_SIZE
                + sprite_layout::ROTATION)
                .into(),
            align: 3,
            memory_index: 0
        }),
        I32Const(ir_target_index),
        LocalGet(local_idx),
        Call(imported_func),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Float]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = true;

crate::instructions_test! {tests; motion_pointindirection; t }
