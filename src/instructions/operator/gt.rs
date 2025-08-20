use wasm_encoder::BlockType;

use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    let block_type = BlockType::Result(ValType::I32);
    Ok(if IrType::QuasiInt.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            wasm![I32GtS]
        } else if IrType::Float.contains(t2) {
            let local1 = func.local(ValType::F64)?;
            let local2 = func.local(ValType::F64)?;
            wasm![
                LocalSet(local2),
                F64ConvertI32S,
                LocalSet(local1),
                LocalGet(local2),
                @isnan(t2),
                If(block_type),
                    I32Const(0), // (number) < NaN always false
                Else,
                    LocalGet(local1),
                    LocalGet(local2),
                    F64Gt,
                End,
            ]
        } else if IrType::String.contains(t2) {
            // TODO: try converting string to number first (if applicable)
            let int2string = func.registries().external_functions().register(
                ("cast", "int2string".into()),
                (vec![ValType::I32], vec![ValType::EXTERNREF]),
            )?;
            let string_gt = func.registries().external_functions().register(
                ("operator", "gt_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let extern_local = func.local(ValType::EXTERNREF)?;
            wasm![
                LocalSet(extern_local),
                Call(int2string),
                LocalGet(extern_local),
                Call(string_gt)
            ]
        } else {
            hq_bug!("bad input")
        }
    } else if IrType::Float.contains(t1) {
        if IrType::Float.contains(t2) {
            let local1 = func.local(ValType::F64)?;
            let local2 = func.local(ValType::F64)?;
            wasm![
                LocalSet(local2),
                LocalTee(local1),
                @isnan(t1),
                If(block_type),
                    LocalGet(local2),
                    @isnan(t2),
                    If(block_type),
                        I32Const(0), // NaN is not greater than NaN
                    Else,
                        I32Const(1), // NaN > number always true
                    End,
                Else,
                    LocalGet(local1),
                    LocalGet(local2),
                    F64Gt,
                End,
            ]
        } else if IrType::QuasiInt.contains(t2) {
            let local1 = func.local(ValType::F64)?;
            let local2 = func.local(ValType::F64)?;
            wasm![
                F64ConvertI32S,
                LocalSet(local2),
                LocalTee(local1),
                @isnan(t1),
                If(block_type),
                    I32Const(1), // NaN > number is always true
                Else,
                    LocalGet(local1),
                    LocalGet(local2),
                    F64Gt,
                End,
            ]
        } else if IrType::String.contains(t2) {
            let float2string = func.registries().external_functions().register(
                ("cast", "float2string".into()),
                (vec![ValType::F64], vec![ValType::EXTERNREF]),
            )?;
            let string_gt = func.registries().external_functions().register(
                ("operator", "gt_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let extern_local = func.local(ValType::EXTERNREF)?;
            wasm![
                LocalSet(extern_local),
                Call(float2string),
                LocalGet(extern_local),
                Call(string_gt)
            ]
        } else {
            hq_bug!("bad input")
        }
    } else if IrType::String.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            // TODO: try converting string to number first
            let int2string = func.registries().external_functions().register(
                ("cast", "int2string".into()),
                (vec![ValType::I32], vec![ValType::EXTERNREF]),
            )?;
            let string_gt = func.registries().external_functions().register(
                ("operator", "gt_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            wasm![Call(int2string), Call(string_gt)]
        } else if IrType::Float.contains(t2) {
            // TODO: try converting string to number first
            let float2string = func.registries().external_functions().register(
                ("cast", "float2string".into()),
                (vec![ValType::F64], vec![ValType::EXTERNREF]),
            )?;
            let string_gt = func.registries().external_functions().register(
                ("operator", "gt_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            wasm![Call(float2string), Call(string_gt)]
        } else if IrType::String.contains(t2) {
            let string_gt = func.registries().external_functions().register(
                ("operator", "gt_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            wasm![Call(string_gt)]
        } else {
            hq_bug!("bad input")
        }
    } else {
        hq_bug!("bad input")
    })
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Any, IrType::Any]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(Singleton(IrType::Boolean))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_lt; t1, t2 ;}
