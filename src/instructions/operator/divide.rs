use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    let f64_local = func.local(ValType::F64)?;
    Ok(wasm![
        LocalSet(f64_local),
        @nanreduce(t1),
        LocalGet(f64_local),
        @nanreduce(t2),
        F64Div
    ])
}

// TODO: is integer division acceptable if we can prove that it will give an integer result (or if it is floored?)
pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::Float, IrType::Float])
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    let t1 = inputs[0];
    let t2 = inputs[1];
    let maybe_positive = (t1.maybe_positive() && t2.maybe_positive())
        || (t1.maybe_negative() && t2.maybe_negative());
    let maybe_negative = (t1.maybe_positive() && t2.maybe_negative())
        || (t1.maybe_negative() && t2.maybe_positive());
    let maybe_zero = t1.maybe_zero() || t1.maybe_nan(); // TODO: can this be narrowed to +/-0?
    let maybe_infinity = t2.maybe_zero() || t2.maybe_nan(); // TODO: can this be narrowed to +/-infinity?
    let maybe_nan = t1.maybe_zero() && t2.maybe_zero();
    Ok(Some(
        IrType::none_if_false(maybe_positive, IrType::FloatPos)
            .or(IrType::none_if_false(maybe_negative, IrType::FloatNeg))
            .or(IrType::none_if_false(maybe_zero, IrType::FloatZero))
            .or(IrType::none_if_false(maybe_infinity, IrType::FloatInf))
            .or(IrType::none_if_false(maybe_nan, IrType::FloatNan)),
    ))
}

pub const YIELDS: bool = false;

crate::instructions_test! {tests; operator_divide; t1, t2 ;}
