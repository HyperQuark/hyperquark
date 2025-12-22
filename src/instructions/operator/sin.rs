use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 1);
    let t1 = inputs[0];
    let imported_func = func.registries().external_functions().register(
        ("operator", "sin".into()),
        (vec![ValType::F64], vec![ValType::F64]),
    )?;
    Ok(wasm![
        @nanreduce(t1),
        F64Const(core::f64::consts::PI.into()),
        F64Mul,
        F64Const(180.0.into()),
        F64Div,
        Call(imported_func),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Float]))
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    hq_assert_eq!(inputs.len(), 1);
    let t1 = inputs[0];
    let maybe_nan = t1.maybe_inf();
    let maybe_real = t1.intersects(IrType::FloatReal.or(IrType::FloatNan));
    Ok(Singleton(
        IrType::none_if_false(maybe_real, IrType::FloatReal)
            .or(IrType::none_if_false(maybe_nan, IrType::FloatNan)),
    ))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_sin; t }
