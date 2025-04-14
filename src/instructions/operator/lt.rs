use wasm_encoder::BlockType;

use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    let block_type = BlockType::Result(ValType::I32);
    Ok(if IrType::QuasiInt.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            wasm![I32LtS]
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
                    I32Const(1), // (number) < NaN always true
                Else,
                    LocalGet(local1),
                    LocalGet(local2),
                    F64Lt,
                End,
            ]
        } else if IrType::String.contains(t2) {
            // TODO: try converting string to number first (if applicable)
            let int2string = func.registries().external_functions().register(
                ("cast", "int2string"),
                (vec![ValType::I32], vec![ValType::EXTERNREF]),
            )?;
            let string_lt = func.registries().external_functions().register(
                ("operator", "lt_string"),
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
                Call(string_lt)
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
                    I32Const(0), // NaN < number always false
                Else,
                    LocalGet(local2),
                    @isnan(t2),
                    If(block_type),
                        I32Const(1), // number < NaN always true
                    Else,
                        LocalGet(local1),
                        LocalGet(local2),
                        F64Lt,
                    End,
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
                    // if t2 is NaN, NaN < NaN is false
                    // if t2 is a number, NaN < (number) is false, since scratch converts to
                    // a string if one or both inputs are NaN, and compares the strings;
                    // numbers (and -) come before N.
                    I32Const(0),
                Else,
                    LocalGet(local1),
                    LocalGet(local2),
                    F64Lt,
                End,
            ]
        } else if IrType::String.contains(t2) {
            let float2string = func.registries().external_functions().register(
                ("cast", "float2string"),
                (vec![ValType::F64], vec![ValType::EXTERNREF]),
            )?;
            let string_lt = func.registries().external_functions().register(
                ("operator", "lt_string"),
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
                Call(string_lt)
            ]
        } else {
            hq_bug!("bad input")
        }
    } else if IrType::String.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            // TODO: try converting string to number first
            let int2string = func.registries().external_functions().register(
                ("cast", "int2string"),
                (vec![ValType::I32], vec![ValType::EXTERNREF]),
            )?;
            let string_lt = func.registries().external_functions().register(
                ("operator", "lt_string"),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            wasm![Call(int2string), Call(string_lt)]
        } else if IrType::Float.contains(t2) {
            // TODO: try converting string to number first
            let float2string = func.registries().external_functions().register(
                ("cast", "float2string"),
                (vec![ValType::F64], vec![ValType::EXTERNREF]),
            )?;
            let string_lt = func.registries().external_functions().register(
                ("operator", "lt_string"),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            wasm![Call(float2string), Call(string_lt)]
        } else if IrType::String.contains(t2) {
            let string_lt = func.registries().external_functions().register(
                ("operator", "lt_string"),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            wasm![Call(string_lt)]
        } else {
            hq_bug!("bad input")
        }
    } else {
        hq_bug!("bad input")
    })
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::Any, IrType::Any])
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    Ok(Some(IrType::Boolean))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_lt; t1, t2 ;}
