use wasm_encoder::MemArg;

use super::super::prelude::*;
use crate::wasm::{mem_layout, StepTarget};

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let ir_target_index: i32 = func
        .target_index()
        .try_into()
        .map_err(|_| make_hq_bug!("target index out of bounds"))?;
    let func_index = func.registries().external_functions().register(
        ("looks", "switchcostumeto".into()),
        (vec![ValType::I32, ValType::I32], vec![]),
    )?;
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        0
    };
    let offset = mem_layout::stage::BLOCK_SIZE
        + wasm_target_index * mem_layout::sprite::BLOCK_SIZE
        + mem_layout::sprite::COSTUME;

    let local_index = func.local(ValType::I32)?;
    Ok(if IrType::QuasiInt.contains(inputs[0]) {
        wasm![
            LocalSet(local_index),
            I32Const(0),
            LocalGet(local_index),
            I32Store(MemArg {
                offset: offset.into(),
                align: 2,
                memory_index: 0,
            }),
            LocalGet(local_index),
            I32Const(ir_target_index),
            Call(func_index),
        ]
    } else {
        hq_todo!("non-integer input types for looks_switchcostumetos")
    })
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    // TODO: accept non-integer values (try to find costume name)
    Ok(Rc::from([IrType::IntPos]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = true;

crate::instructions_test! {tests; looks_switchcostumeto; t ; }
