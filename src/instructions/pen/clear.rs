use wasm_encoder::MemArg;

use super::super::prelude::*;
use crate::wasm::{StepTarget, mem_layout};

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let func_index = func.registries().external_functions().register(
        ("pen", "clear".into()),
        (vec![], vec![]),
    )?;
    Ok(wasm![
        Call(func_index),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = true;

crate::instructions_test! {tests; pen_clear; ; }
