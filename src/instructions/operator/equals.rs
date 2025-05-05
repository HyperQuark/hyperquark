use wasm_encoder::BlockType;

use super::super::prelude::*;

#[expect(clippy::too_many_lines, reason = "long monomorphisation routine")]
pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    let block_type = BlockType::Result(ValType::I32);
    Ok(if IrType::QuasiInt.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            wasm![I32Eq]
        } else if IrType::Float.contains(t2) {
            let local1 = func.local(ValType::F64)?;
            let local2 = func.local(ValType::F64)?;
            wasm![
                LocalSet(local2),
                F64ConvertI32S,
                LocalSet(local1),
                LocalGet(local2),
                @isnan(t1),
                If(block_type),
                    LocalGet(local2),
                    @isnan(t2),
                    If(block_type),
                        I32Const(1), // NaN == NaN (in scratch)
                    Else,
                        I32Const(0), // NaN != number
                    End,
                Else,
                    LocalGet(local2),
                    @isnan(t2),
                    If(block_type),
                        I32Const(0), // number != NaN
                    Else,
                        LocalGet(local1),
                        LocalGet(local2),
                        F64Eq,
                    End,
                End,
            ]
        } else if IrType::String.contains(t2) {
            // TODO: try converting string to number first (if applicable)
            let int2string = func.registries().external_functions().register(
                ("cast", "int2string".into()),
                (vec![ValType::I32], vec![ValType::EXTERNREF]),
            )?;
            let string_eq = func.registries().external_functions().register(
                // we can't use wasm:js-string/equals because scratch converts to lowercase first
                ("operator", "eq_string".into()),
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
                Call(string_eq)
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
                        I32Const(1), // NaN == NaN (in scratch)
                    Else,
                        I32Const(0), // NaN != number
                    End,
                Else,
                    LocalGet(local2),
                    @isnan(t2),
                    If(block_type),
                        I32Const(0), // number != NaN
                    Else,
                        LocalGet(local1),
                        LocalGet(local2),
                        F64Eq,
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
                LocalGet(local2),
                    @isnan(t2),
                    If(block_type),
                        I32Const(1), // NaN == NaN (in scratch)
                    Else,
                        I32Const(0), // NaN != number
                    End,
                Else,
                    LocalGet(local2),
                    @isnan(t2),
                    If(block_type),
                        I32Const(0), // number != NaN
                    Else,
                        LocalGet(local1),
                        LocalGet(local2),
                        F64Eq,
                    End,
                End,
            ]
        } else if IrType::String.contains(t2) {
            // TODO: try converting string to number first
            let float2string = func.registries().external_functions().register(
                ("cast", "float2string".into()),
                (vec![ValType::F64], vec![ValType::EXTERNREF]),
            )?;
            let string_eq = func.registries().external_functions().register(
                ("operator", "eq_string".into()),
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
                Call(string_eq)
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
            let string_eq = func.registries().external_functions().register(
                ("operator", "eq_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            wasm![Call(int2string), Call(string_eq)]
        } else if IrType::Float.contains(t2) {
            // TODO: try converting string to number first
            let float2string = func.registries().external_functions().register(
                ("cast", "float2string".into()),
                (vec![ValType::F64], vec![ValType::EXTERNREF]),
            )?;
            let string_eq = func.registries().external_functions().register(
                ("operator", "eq_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            wasm![Call(float2string), Call(string_eq)]
        } else if IrType::String.contains(t2) {
            // TODO: try converting string to number first
            let string_eq = func.registries().external_functions().register(
                ("operator", "eq_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            wasm![Call(string_eq)]
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

crate::instructions_test! {tests; operator_equals; t1, t2 ;}
