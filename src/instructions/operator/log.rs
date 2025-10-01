use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let imported_func = func.registries().external_functions().register(
        ("operator", "log".into()),
        (vec![ValType::F64], vec![ValType::F64]),
    )?;
    let t1 = inputs[0];
    Ok(wasm![
        @nanreduce(t1),
        Call(imported_func),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Float]))
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    hq_assert_eq!(inputs.len(), 1);
    let t1 = inputs[0];
    let maybe_nan = t1.maybe_negative();
    let maybe_real = t1.intersects(IrType::FloatPosReal);
    let maybe_pos_inf = t1.intersects(IrType::FloatPosInf);
    let maybe_neg_inf = t1.maybe_zero() || t1.maybe_nan();
    Ok(Singleton(
        IrType::none_if_false(maybe_real, IrType::FloatReal)
            .or(IrType::none_if_false(maybe_pos_inf, IrType::FloatPosInf))
            .or(IrType::none_if_false(maybe_neg_inf, IrType::FloatNegInf))
            .or(IrType::none_if_false(maybe_nan, IrType::FloatNan)),
    ))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_log; t }
