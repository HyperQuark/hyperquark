use wasm_encoder::BlockType as WasmBlockType;

use super::super::prelude::*;

#[expect(clippy::too_many_lines, reason = "wasm generation is just long")]
pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    let block_type = WasmBlockType::Result(ValType::I32);
    Ok(if IrType::Int.contains(t1) {
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
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let float_local = func.local(ValType::F64)?;
            wasm![
                Call(string2float),
                LocalSet(float_local),
                F64ConvertI32S,
                LocalGet(float_local),
                F64Eq,
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
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let string_eq = func.registries().external_functions().register(
                ("operator", "eq_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let string_local = func.local(ValType::EXTERNREF)?;
            let float_local = func.local(ValType::F64)?;
            let nan_string = func.registries().strings().register_default("nan".into())?;
            wasm![
                LocalSet(string_local),
                LocalTee(float_local),
                @isnan(t1),
                If(block_type),
                    GlobalGet(nan_string),
                    LocalGet(string_local),
                    Call(string_eq),
                Else,
                    LocalGet(string_local),
                    Call(string2float),
                    LocalGet(float_local),
                    F64Eq,
                End,
            ]
        } else {
            hq_bug!("bad input")
        }
    } else if IrType::Boolean.contains(t1) {
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
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let string_eq = func.registries().external_functions().register(
                ("wasm:js-string", "equals".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let bool_local = func.local(ValType::I32)?;
            let string_local = func.local(ValType::EXTERNREF)?;
            let float_local = func.local(ValType::F64)?;
            let float_type = IrType::Float;
            let true_string = func
                .registries()
                .strings()
                .register_default("true".into())?;
            let false_string = func
                .registries()
                .strings()
                .register_default("false".into())?;
            wasm![
                LocalTee(string_local),
                Call(string2float),
                LocalSet(float_local),
                LocalSet(bool_local),
                LocalGet(float_local),
                @isnan(float_type),
                If(block_type),
                    GlobalGet(true_string),
                    GlobalGet(false_string),
                    LocalGet(bool_local),
                    TypedSelect(ValType::EXTERNREF),
                    LocalGet(string_local),
                    Call(string_eq),
                Else,
                    LocalGet(float_local),
                    LocalGet(bool_local),
                    F64ConvertI32S,
                    F64Eq,
                End,
            ]
        } else {
            hq_bug!("bad input")
        }
    } else if IrType::String.contains(t1) {
        if IrType::Int.contains(t2) {
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let int_local = func.local(ValType::I32)?;
            wasm![
                LocalSet(int_local),
                Call(string2float),
                LocalGet(int_local),
                F64ConvertI32S,
                F64Eq
            ]
        } else if IrType::Boolean.contains(t2) {
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let string_eq = func.registries().external_functions().register(
                ("wasm:js-string", "equals".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let bool_local = func.local(ValType::I32)?;
            let string_local = func.local(ValType::EXTERNREF)?;
            let float_local = func.local(ValType::F64)?;
            let float_type = IrType::Float;
            let true_string = func
                .registries()
                .strings()
                .register_default("true".into())?;
            let false_string = func
                .registries()
                .strings()
                .register_default("false".into())?;
            wasm![
                LocalSet(bool_local),
                LocalTee(string_local),
                Call(string2float),
                LocalTee(float_local),
                @isnan(float_type),
                If(block_type),
                    GlobalGet(true_string),
                    GlobalGet(false_string),
                    LocalGet(bool_local),
                    TypedSelect(ValType::EXTERNREF),
                    LocalGet(string_local),
                    Call(string_eq),
                Else,
                    LocalGet(float_local),
                    LocalGet(bool_local),
                    F64ConvertI32S,
                    F64Eq,
                End,
            ]
        } else if IrType::Float.contains(t2) {
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let string_eq = func.registries().external_functions().register(
                ("operator", "eq_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let string_local = func.local(ValType::EXTERNREF)?;
            let float_local = func.local(ValType::F64)?;
            let nan_string = func.registries().strings().register_default("nan".into())?;
            wasm![
                LocalSet(float_local),
                LocalSet(string_local),
                LocalGet(float_local),
                @isnan(t2),
                If(block_type),
                    GlobalGet(nan_string),
                    LocalGet(string_local),
                    Call(string_eq),
                Else,
                    LocalGet(string_local),
                    Call(string2float),
                    LocalGet(float_local),
                    F64Eq,
                End,
            ]
        } else if IrType::String.contains(t2) {
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let string_eq = func.registries().external_functions().register(
                ("operator", "eq_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let local1 = func.local(ValType::EXTERNREF)?;
            let local2 = func.local(ValType::EXTERNREF)?;
            let local3 = func.local(ValType::F64)?;
            let local4 = func.local(ValType::F64)?;
            let float_type = IrType::Float;
            wasm![
                LocalSet(local2),
                LocalTee(local1),
                Call(string2float),
                LocalTee(local3),
                @isnan(float_type),
                LocalGet(local2),
                Call(string2float),
                LocalTee(local4),
                @isnan(float_type),
                I32And,
                If(block_type),
                    LocalGet(local1),
                    LocalGet(local2),
                    Call(string_eq),
                Else,
                    LocalGet(local3),
                    LocalGet(local4),
                    F64Eq,
                End,
            ]
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

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

crate::instructions_test! {tests; operator_equals; t1, t2 ;}
