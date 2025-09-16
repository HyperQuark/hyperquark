use wasm_encoder::MemArg;

use super::super::prelude::*;
use crate::wasm::{StepTarget, mem_layout};

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let ir_target_index: i32 = func
        .target_index()
        .try_into()
        .map_err(|_| make_hq_bug!("target index out of bounds"))?;
    let func_index = func.registries().external_functions().register(
        ("looks", "setvisible".into()),
        (vec![ValType::I32, ValType::I32], vec![]),
    )?;
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        hq_bad_proj!("looks_setvisible called in stage")
    };
    let local_index = func.local(ValType::I32)?;
    Ok(wasm![
        LocalSet(local_index),
        I32Const(0),
        LocalGet(local_index),
        I32Store8(MemArg {
            offset: (mem_layout::stage::BLOCK_SIZE
                + wasm_target_index * mem_layout::sprite::BLOCK_SIZE
                + mem_layout::sprite::VISIBLE)
                .into(),
            align: 0,
            memory_index: 0,
        }),
        LocalGet(local_index),
        I32Const(ir_target_index),
        Call(func_index),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Boolean]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = true;

crate::instructions_test! {tests; looks_setvisible; t ; }
