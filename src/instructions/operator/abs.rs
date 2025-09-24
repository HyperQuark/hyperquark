use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 1);
    let t1 = inputs[0];
    Ok(if IrType::QuasiInt.contains(t1) {
        if IrType::IntPos
            .or(IrType::IntZero)
            .or(IrType::Boolean)
            .contains(t1)
        {
            wasm![]
        } else {
            let local_index = func.local(ValType::I32)?;
            wasm![
                LocalTee(local_index),
                LocalGet(local_index),
                I32Const(31),
                I32ShrS,
                I32Xor,
                LocalGet(local_index),
                I32Const(31),
                I32ShrS,
                I32Sub,
            ]
        }
    } else if IrType::Float.contains(t1) {
        if IrType::FloatPos.contains(t1) {
            wasm![]
        } else {
            wasm![
                @nanreduce(t1),
                F64Abs,
            ]
        }
    } else {
        hq_bug!("bad input: {:?}", inputs)
    })
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Number]))
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    hq_assert_eq!(inputs.len(), 1);
    let t1 = inputs[0];
    let maybe_zero = t1.maybe_zero() || t1.maybe_nan();
    let maybe_real = t1.intersects(
        IrType::FloatReal
            .or(IrType::IntNonZero)
            .or(IrType::BooleanTrue),
    );
    Ok(Singleton(if IrType::QuasiInt.contains(t1) {
        IrType::none_if_false(maybe_real, IrType::IntPos)
            .or(IrType::none_if_false(maybe_zero, IrType::IntZero))
    } else {
        IrType::none_if_false(maybe_real, IrType::FloatPosReal)
            .or(IrType::none_if_false(maybe_zero, IrType::FloatZero))
            .or(IrType::none_if_false(
                IrType::FloatInf.intersects(t1),
                IrType::FloatPosInf,
            ))
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_abs; t }
