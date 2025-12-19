//! this is called modulo rather than mod, just because mod.rs has a special meaning in rust

use super::super::prelude::*;
use wasm_encoder::BlockType as WasmBlockType;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    // crate::log!("{t2}");
    Ok(if IrType::QuasiInt.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            let modulus_local = func.local(ValType::I32)?;
            let result_local = func.local(ValType::I32)?;
            if t2.maybe_zero() {
                let if_block_type = func
                    .registries()
                    .types()
                    .function(vec![ValType::I32, ValType::I32], vec![ValType::F64])?;
                wasm![
                    LocalTee(modulus_local),
                    LocalGet(modulus_local),
                    I32Eqz,
                    If(WasmBlockType::FunctionType(if_block_type)),
                    Drop,
                    Drop,
                    F64Const(f64::NAN.into()),
                    Else,
                    I32RemS,
                    LocalTee(result_local),
                    LocalGet(modulus_local),
                    I32Const(0),
                    LocalGet(result_local),
                    LocalGet(modulus_local),
                    I32DivS,
                    I32Const(0),
                    I32LtS,
                    Select,
                    I32Add,
                    F64ConvertI32S,
                    End,
                ]
            } else {
                wasm![
                    LocalTee(modulus_local),
                    I32RemS,
                    LocalTee(result_local),
                    LocalGet(modulus_local),
                    I32Const(0),
                    LocalGet(result_local),
                    LocalGet(modulus_local),
                    I32DivS,
                    I32Const(0),
                    I32LtS,
                    Select,
                    I32Add,
                ]
            }
        } else if IrType::Float.contains(t2) {
            let modulus_local = func.local(ValType::F64)?;
            let n_local = func.local(ValType::F64)?;
            let div_local = func.local(ValType::F64)?;
            wasm![
                @nanreduce(t2),
                LocalSet(modulus_local),
                F64ConvertI32S,
                LocalTee(n_local),
                LocalGet(n_local),
                LocalGet(modulus_local),
                F64Div,
                LocalTee(div_local),
                F64Trunc,
                LocalGet(modulus_local),
                F64Mul,
                F64Sub,
                LocalGet(modulus_local),
                F64Const(0.0.into()),
                LocalGet(div_local),
                F64Const(0.0.into()),
                F64Lt,
                Select,
                F64Add,
            ]
        } else {
            hq_bug!("bad input")
        }
    } else if IrType::Float.contains(t1) {
        let modulus_local = func.local(ValType::F64)?;
        let n_local = func.local(ValType::F64)?;
        let div_local = func.local(ValType::F64)?;
        if IrType::Float.contains(t2) {
            wasm![
                @nanreduce(t2),
                LocalSet(modulus_local),
                @nanreduce(t1),
                LocalTee(n_local),
                LocalGet(n_local),
                LocalGet(modulus_local),
                F64Div,
                LocalTee(div_local),
                F64Trunc,
                LocalGet(modulus_local),
                F64Mul,
                F64Sub,
                LocalGet(modulus_local),
                F64Const(0.0.into()),
                LocalGet(div_local),
                F64Const(0.0.into()),
                F64Lt,
                Select,
                F64Add,
            ]
        } else if IrType::QuasiInt.contains(t2) {
            wasm![
                F64ConvertI32S,
                LocalSet(modulus_local),
                @nanreduce(t1),
                LocalTee(n_local),
                LocalGet(n_local),
                LocalGet(modulus_local),
                F64Div,
                LocalTee(div_local),
                F64Trunc,
                LocalGet(modulus_local),
                F64Mul,
                F64Sub,
                LocalGet(modulus_local),
                F64Const(0.0.into()),
                LocalGet(div_local),
                F64Const(0.0.into()),
                F64Lt,
                Select,
                F64Add,
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
    let maybe_positive = t2.maybe_positive() || (t2.maybe_inf() && t1.maybe_positive());
    let maybe_negative = t2.maybe_negative() || (t2.maybe_inf() && t1.maybe_negative());
    let maybe_zero = t1.maybe_zero() || t1.maybe_nan();
    let maybe_nan = t1.maybe_inf() || t2.maybe_nan() || t2.maybe_zero();
    // TODO: benchmark whether it is better to always convert to a float for int % int if we cannot
    // guarantee it to be non-nan, or whether to box it so we use integer operations where possible
    Ok(Singleton(
        if IrType::QuasiInt.contains(t1.or(t2)) && !maybe_nan {
            IrType::none_if_false(maybe_positive, IrType::IntPos)
                .or(IrType::none_if_false(maybe_negative, IrType::IntNeg))
                .or(IrType::none_if_false(maybe_zero, IrType::IntZero))
        } else {
            IrType::none_if_false(maybe_positive, IrType::FloatPos)
                .or(IrType::none_if_false(maybe_negative, IrType::FloatNeg))
                .or(IrType::none_if_false(maybe_zero, IrType::FloatZero))
                .or(IrType::none_if_false(maybe_nan, IrType::FloatNan))
        },
    ))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_modulo; t1, t2 ;}
