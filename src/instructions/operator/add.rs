use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    Ok(if IrType::QuasiInt.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            wasm![I32Add]
        } else if IrType::Float.contains(t2) {
            let f64_local = func.local(ValType::F64)?;
            wasm![
                LocalSet(f64_local),
                F64ConvertI32S,
                LocalGet(f64_local),
                @nanreduce(t2),
                F64Add,
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
                F64Add
            ]
        } else if IrType::QuasiInt.contains(t2) {
            let i32_local = func.local(ValType::I32)?;
            wasm![
                LocalSet(i32_local),
                @nanreduce(t1),
                LocalGet(i32_local),
                F64ConvertI32S,
                F64Add
            ]
        } else {
            hq_bug!("bad input")
        }
    } else {
        hq_bug!("bad input: {:?}", inputs)
    })
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Number, IrType::Number]))
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    hq_assert_eq!(inputs.len(), 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    let maybe_positive = t1.maybe_positive() || t2.maybe_positive();
    let maybe_negative = t1.maybe_negative() || t2.maybe_negative();
    let maybe_zero = (t1.maybe_zero() || t1.maybe_nan()) && (t2.maybe_zero() || t2.maybe_nan());
    let maybe_nan = (IrType::FloatNegInf.intersects(t1) && IrType::FloatPosInf.intersects(t2))
        || (IrType::FloatNegInf.intersects(t2) && IrType::FloatPosInf.intersects(t1));
    Ok(Singleton(if IrType::QuasiInt.contains(t1.or(t2)) {
        IrType::none_if_false(maybe_positive, IrType::IntPos)
            .or(IrType::none_if_false(maybe_negative, IrType::IntNeg))
            .or(IrType::none_if_false(maybe_zero, IrType::IntZero))
    } else if (IrType::QuasiInt.contains(t1) && IrType::Float.contains(t2))
        || (IrType::QuasiInt.contains(t2) && IrType::Float.contains(t1))
        || IrType::Float.contains(t1.or(t2))
    {
        IrType::none_if_false(maybe_positive, IrType::FloatPos)
            .or(IrType::none_if_false(maybe_negative, IrType::FloatNeg))
            .or(IrType::none_if_false(maybe_zero, IrType::FloatZero))
            .or(IrType::none_if_false(maybe_nan, IrType::FloatNan))
    } else {
        // there is a boxed type somewhere
        // TODO: can these bounds be tightened? e.g. it may only be a positive int or negative float?
        // i have no idea if that would ever work, but it would be useful for considering when
        // addition/subtraction may give NaN (since inf-inf=nan but inf+inf=inf)
        IrType::none_if_false(maybe_positive, IrType::FloatPos.or(IrType::IntPos))
            .or(IrType::none_if_false(
                maybe_negative,
                IrType::FloatNeg.or(IrType::IntNeg),
            ))
            .or(IrType::none_if_false(
                maybe_zero,
                IrType::FloatZero.or(IrType::IntZero),
            ))
            .or(IrType::none_if_false(maybe_nan, IrType::FloatNan))
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_add; t1, t2 ;}
