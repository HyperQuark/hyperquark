use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 1);
    let t1 = inputs[0];
    Ok(
        if IrType::QuasiInt
            .or(IrType::FloatInt)
            .or(IrType::FloatInf)
            .contains(t1)
        {
            wasm![]
        } else if IrType::FloatNan.contains(t1) {
            wasm![Drop, F64Const(0.0)]
        } else if IrType::Float.contains(t1) {
            wasm![
                @nanreduce(t1),
                F64Floor,
            ]
        } else {
            hq_bug!("bad input: {:?}", inputs)
        },
    )
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Number]))
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    hq_assert_eq!(inputs.len(), 1);
    let t1 = inputs[0];
    let maybe_positive = t1.maybe_positive();
    let maybe_negative = t1.maybe_negative();
    let maybe_zero = maybe_positive || t1.maybe_zero() || t1.maybe_nan();
    Ok(Singleton(if IrType::QuasiInt.contains(t1) {
        IrType::none_if_false(maybe_positive, IrType::IntPos)
            .or(IrType::none_if_false(maybe_negative, IrType::IntNeg))
            .or(IrType::none_if_false(t1.maybe_zero(), IrType::IntZero))
    } else {
        IrType::none_if_false(maybe_positive, IrType::FloatPosInt)
            .or(IrType::none_if_false(maybe_negative, IrType::FloatNegInt))
            .or(IrType::none_if_false(maybe_zero, IrType::FloatZero))
            .or(IrType::none_if_false(
                IrType::FloatPosInf.intersects(t1),
                IrType::FloatPosInf,
            ))
            .or(IrType::none_if_false(
                IrType::FloatNegInf.intersects(t1),
                IrType::FloatNegInf,
            ))
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_floor; t }
