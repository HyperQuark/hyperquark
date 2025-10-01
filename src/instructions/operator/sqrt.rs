use super::super::prelude::*;

pub fn wasm(_func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    Ok(wasm![F64Sqrt])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Float]))
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    hq_assert_eq!(inputs.len(), 1);
    let t1 = inputs[0];
    let maybe_zero = t1.maybe_zero() || t1.maybe_nan();
    let maybe_nan = t1.maybe_negative();
    let maybe_pos_real = t1.intersects(IrType::FloatPosReal);
    let maybe_inf = t1.intersects(IrType::FloatPosInf);
    Ok(Singleton(
        IrType::none_if_false(maybe_pos_real, IrType::FloatPosReal)
            .or(IrType::none_if_false(maybe_zero, IrType::FloatZero))
            .or(IrType::none_if_false(maybe_inf, IrType::FloatPosInf))
            .or(IrType::none_if_false(maybe_nan, IrType::FloatNan)),
    ))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_sqrt; t }
