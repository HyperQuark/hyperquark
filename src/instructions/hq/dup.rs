use crate::wasm::WasmProject;

use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let local = func.local(WasmProject::ir_type_to_wasm(inputs[0])?)?;
    Ok(wasm![LocalTee(local), LocalGet(local),])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Any]))
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(MultiValue(Rc::from([inputs[0], inputs[0]])))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; hq_dup; t}
