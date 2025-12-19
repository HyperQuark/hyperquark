use super::super::prelude::*;
use wasm_encoder::BlockType as WasmBlockType;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    let from_local = func.local(ValType::F64)?;
    let to_local = func.local(ValType::F64)?;
    let low_local = func.local(ValType::F64)?;
    let high_local = func.local(ValType::F64)?;
    let imported_function = func
        .registries()
        .external_functions()
        .register(("operator", "random".into()), (vec![], vec![ValType::F64]))?;
    Ok(if IrType::QuasiInt.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            wasm![
                F64ConvertI32S,
                LocalSet(to_local),
                F64ConvertI32S,
                LocalTee(from_local)
            ]
        } else if IrType::Float.contains(t2) {
            wasm![
                @nanreduce(t2),
                LocalSet(to_local),
                F64ConvertI32S,
                LocalTee(from_local),
            ]
        } else {
            hq_bug!("bad input")
        }
    } else if IrType::Float.contains(t1) {
        if IrType::Float.contains(t2) {
            wasm![
                @nanreduce(t2),
                LocalSet(to_local),
                @nanreduce(t1),
                LocalTee(from_local),
            ]
        } else if IrType::QuasiInt.contains(t2) {
            wasm![
                F64ConvertI32S,
                LocalSet(to_local),
                @nanreduce(t1),
                LocalTee(from_local),
            ]
        } else {
            hq_bug!("bad input")
        }
    } else {
        hq_bug!("bad input: {:?}", inputs)
    }
    .into_iter()
    .chain(wasm![
        LocalGet(to_local),
        LocalGet(from_local),
        LocalGet(to_local),
        F64Le,
        Select,
        LocalSet(low_local),
        LocalGet(from_local),
        LocalGet(to_local),
        LocalGet(from_local),
        LocalGet(to_local),
        F64Ge,
        Select,
        LocalSet(high_local),
        Block(WasmBlockType::Result(ValType::F64)),
        LocalGet(low_local),
        LocalGet(low_local),
        LocalGet(high_local),
        F64Eq,
        BrIf(0),
    ])
    .chain(
        if IrType::QuasiInt
            .or(IrType::FloatInt)
            .or(IrType::FloatZero)
            .contains(t1.or(t2))
        {
            wasm![
                Call(imported_function),
                LocalGet(high_local),
                F64Const(1.0.into()),
                F64Add,
                LocalGet(low_local),
                F64Sub,
                F64Mul,
                F64Floor,
                F64Add,
            ]
        } else if IrType::QuasiInt
            .or(IrType::FloatInt)
            .or(IrType::FloatZero)
            .intersects(t1.or(t2))
        {
            wasm![
                I64TruncSatF64S,
                F64ConvertI64S,
                LocalGet(low_local),
                F64Eq,
                LocalGet(high_local),
                I64TruncSatF64S,
                F64ConvertI64S,
                LocalGet(high_local),
                F64Eq,
                I32And,
                If(WasmBlockType::Result(ValType::F64)),
                Call(imported_function),
                LocalGet(high_local),
                F64Const(1.0.into()),
                F64Add,
                LocalGet(low_local),
                F64Sub,
                F64Mul,
                F64Floor,
                Else,
                Call(imported_function),
                LocalGet(high_local),
                LocalGet(low_local),
                F64Sub,
                F64Mul,
                End,
                LocalGet(low_local),
                F64Add,
            ]
        } else {
            wasm![
                Call(imported_function),
                LocalGet(high_local),
                LocalGet(low_local),
                F64Sub,
                F64Mul,
                F64Add,
            ]
        },
    )
    .chain(wasm![End,])
    .chain(if IrType::QuasiInt.contains(t1.or(t2)) {
        wasm![I32TruncSatF64S]
    } else {
        wasm![]
    })
    .collect())
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
    let maybe_zero = (t1.maybe_negative() && t2.maybe_positive())
        || (t1.maybe_positive() && t2.maybe_negative())
        || t1.or(t2).maybe_zero()
        || t1.or(t2).maybe_nan();
    let maybe_nan = (IrType::FloatNegInf.intersects(t1) && IrType::FloatPosInf.intersects(t2))
        || (IrType::FloatNegInf.intersects(t2) && IrType::FloatPosInf.intersects(t1));
    let maybe_pos_inf = t1.or(t2).intersects(IrType::FloatPosInf);
    let maybe_neg_inf = t1.or(t2).intersects(IrType::FloatNegInf);
    Ok(Singleton(if IrType::QuasiInt.contains(t1.or(t2)) {
        IrType::none_if_false(maybe_positive, IrType::IntPos)
            .or(IrType::none_if_false(maybe_negative, IrType::IntNeg))
            .or(IrType::none_if_false(maybe_zero, IrType::IntZero))
    } else {
        IrType::none_if_false(maybe_positive, IrType::FloatPos)
            .or(IrType::none_if_false(maybe_negative, IrType::FloatNeg))
            .or(IrType::none_if_false(maybe_zero, IrType::FloatZero))
            .or(IrType::none_if_false(maybe_nan, IrType::FloatNan))
            .or(IrType::none_if_false(maybe_pos_inf, IrType::FloatPosInf))
            .or(IrType::none_if_false(maybe_neg_inf, IrType::FloatNegInf))
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_random; t1, t2 ;}
