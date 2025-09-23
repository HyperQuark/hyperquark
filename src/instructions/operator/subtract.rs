use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    Ok(if IrType::QuasiInt.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            wasm![I32Sub]
        } else if IrType::Float.contains(t2) {
            let f64_local = func.local(ValType::F64)?;
            wasm![
                LocalSet(f64_local),
                F64ConvertI32S,
                LocalGet(f64_local),
                @nanreduce(t2),
                F64Sub,
            ]
        } else {
            hq_bug!("bad input")
        }
    } else if IrType::Float.contains(t1) {
        if IrType::Float.contains(t2) {
            let f64_local = func.local(ValType::F64)?;
            wasm![
                @nanreduce(t2),
                LocalSet(f64_local),
                @nanreduce(t1),
                LocalGet(f64_local),
                F64Sub
            ]
        } else if IrType::QuasiInt.contains(t2) {
            let i32_local = func.local(ValType::I32)?;
            wasm![
                LocalSet(i32_local),
                @nanreduce(t1),
                LocalGet(i32_local),
                F64ConvertI32S,
                F64Sub
            ]
        } else {
            hq_bug!("bad input")
        }
    } else {
        hq_bug!("bad input")
    })
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Number, IrType::Number]))
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    hq_assert!(inputs.len() == 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    let maybe_positive = t1.maybe_positive() || t2.maybe_negative();
    let maybe_negative = t1.maybe_negative() || t2.maybe_positive();
    let maybe_zero = ((t1.maybe_zero() || t1.maybe_nan()) && (t2.maybe_zero() || t2.maybe_nan()))
        || (t1.maybe_positive() && t2.maybe_positive())
        || (t1.maybe_negative() && t2.maybe_negative());
    let maybe_nan =
        IrType::FloatPosInf.intersects(t1.and(t2)) || IrType::FloatNegInf.intersects(t1.and(t2));
    Ok(Singleton(if IrType::QuasiInt.contains(t1.or(t2)) {
        IrType::none_if_false(maybe_positive, IrType::IntPos)
            .or(IrType::none_if_false(maybe_negative, IrType::IntNeg))
            .or(IrType::none_if_false(maybe_zero, IrType::IntZero))
    } else {
        IrType::none_if_false(maybe_positive, IrType::FloatPos)
            .or(IrType::none_if_false(maybe_negative, IrType::FloatNeg))
            .or(IrType::none_if_false(maybe_zero, IrType::FloatZero))
            .or(IrType::none_if_false(maybe_nan, IrType::FloatNan))
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_subtract; t1, t2 ;}
