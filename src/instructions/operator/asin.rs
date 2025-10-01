use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 1);
    let t1 = inputs[0];
    let imported_func = func.registries().external_functions().register(
        ("operator", "asin".into()),
        (vec![ValType::F64], vec![ValType::F64]),
    )?;
    Ok(wasm![
        @nanreduce(t1),
        Call(imported_func),
        F64Const(core::f64::consts::PI),
        F64Div,
        F64Const(180.0),
        F64Mul,
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Float]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(Singleton(IrType::Float))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_asin; t }
