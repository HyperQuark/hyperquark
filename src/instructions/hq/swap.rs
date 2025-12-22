use crate::wasm::WasmProject;

use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert!(inputs.len() == 2);
    let local1 = func.local(WasmProject::ir_type_to_wasm(inputs[0])?)?;
    let local2 = func.local(WasmProject::ir_type_to_wasm(inputs[1])?)?;
    Ok(wasm![
        LocalSet(local2),
        LocalSet(local1),
        LocalGet(local2),
        LocalGet(local1),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Any, IrType::Any]))
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(MultiValue(Rc::from([inputs[1], inputs[0]])))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; hq_swap; t1, t2}
