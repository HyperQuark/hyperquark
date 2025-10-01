use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let imported_func = func.registries().external_functions().register(
        ("operator", "pow10".into()),
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
    let maybe_pos = t1.intersects(IrType::FloatReal) || t1.maybe_nan();
    let maybe_pos_inf = t1.intersects(IrType::FloatPosInf);
    let maybe_zero = t1.intersects(IrType::FloatNegInf);
    Ok(Singleton(
        IrType::none_if_false(maybe_pos, IrType::FloatPosReal)
            .or(IrType::none_if_false(maybe_pos_inf, IrType::FloatPosInf))
            .or(IrType::none_if_false(maybe_zero, IrType::FloatZero)),
    ))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_pow10; t }
