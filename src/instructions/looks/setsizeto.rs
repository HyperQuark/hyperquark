use wasm_encoder::MemArg;

use super::super::prelude::*;
use crate::wasm::{StepTarget, mem_layout};

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let t1 = inputs[0];
    let ir_target_index: i32 = func
        .target_index()
        .try_into()
        .map_err(|_| make_hq_bug!("target index out of bounds"))?;
    let func_index = func.registries().external_functions().register(
        ("looks", "setsizeto".into()),
        (vec![ValType::F64, ValType::I32], vec![]),
    )?;
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        hq_bad_proj!("looks_setsizeto called in stage")
    };
    let local_index = func.local(ValType::F64)?;
    Ok(wasm![
        @nanreduce(t1),
        LocalSet(local_index),
        I32Const(0),
        LocalGet(local_index),
        F64Store(MemArg {
            offset: (mem_layout::stage::BLOCK_SIZE
                + wasm_target_index * mem_layout::sprite::BLOCK_SIZE
                + mem_layout::sprite::SIZE)
                .into(),
            align: 3,
            memory_index: 0,
        }),
        LocalGet(local_index),
        I32Const(ir_target_index),
        Call(func_index),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Float]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = true;

crate::instructions_test! {tests; looks_setsizeto; t ; }
