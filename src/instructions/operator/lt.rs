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
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let int2string = func.registries().external_functions().register(
                ("cast", "int2string".into()),
                (vec![ValType::I32], vec![ValType::EXTERNREF]),
            )?;
            let string_lt = func.registries().external_functions().register(
                ("operator", "lt_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let extern_local = func.local(ValType::EXTERNREF)?;
            let float_local = func.local(ValType::F64)?;
            let block_type = func
                .registries()
                .types()
                .function(vec![ValType::I32], vec![ValType::I32])?;
            wasm![
                LocalTee(extern_local),
                Call(string2float),
                LocalTee(float_local),
                LocalGet(float_local),
                F64Ne, // test if float_local is NaN
                If(BlockType::FunctionType(block_type)),
                Call(int2string),
                LocalGet(extern_local),
                Call(string_lt),
                Else,
                F64ConvertI32S,
                LocalGet(float_local),
                F64Lt,
                End,
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
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let float2string = func.registries().external_functions().register(
                ("cast", "float2string".into()),
                (vec![ValType::F64], vec![ValType::EXTERNREF]),
            )?;
            let string_lt = func.registries().external_functions().register(
                ("operator", "lt_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let extern_local = func.local(ValType::EXTERNREF)?;
            let float1_local = func.local(ValType::F64)?;
            let float2_local = func.local(ValType::F64)?;
            let block_type = func
                .registries()
                .types()
                .function(vec![ValType::F64], vec![ValType::I32])?;
            wasm![
                LocalSet(extern_local),
                LocalTee(float1_local),
                LocalGet(extern_local),
                Call(string2float),
                LocalTee(float2_local),
                LocalGet(float2_local),
                F64Ne, // test if float_local is NaN
                LocalGet(float1_local),
                @isnan(t1),
                I32Or,
                If(BlockType::FunctionType(block_type)),
                Call(float2string),
                LocalGet(extern_local),
                Call(string_lt),
                Else,
                LocalGet(float2_local),
                F64Lt,
                End,
            ]
        } else {
            hq_bug!("bad input")
        }
    } else if IrType::String.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let int2string = func.registries().external_functions().register(
                ("cast", "int2string".into()),
                (vec![ValType::I32], vec![ValType::EXTERNREF]),
            )?;
            let string_lt = func.registries().external_functions().register(
                ("operator", "lt_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let extern_local = func.local(ValType::EXTERNREF)?;
            let float_local = func.local(ValType::F64)?;
            let int_local = func.local(ValType::I32)?;
            wasm![
                LocalSet(int_local),
                LocalTee(extern_local),
                Call(string2float),
                LocalTee(float_local),
                LocalGet(float_local),
                F64Ne, // test if float_local is NaN
                If(BlockType::Result(ValType::I32)),
                LocalGet(extern_local),
                LocalGet(int_local),
                Call(int2string),
                Call(string_lt),
                Else,
                LocalGet(float_local),
                LocalGet(int_local),
                F64ConvertI32S,
                F64Lt,
                End,
            ]
        } else if IrType::Float.contains(t2) {
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let float2string = func.registries().external_functions().register(
                ("cast", "float2string".into()),
                (vec![ValType::F64], vec![ValType::EXTERNREF]),
            )?;
            let string_lt = func.registries().external_functions().register(
                ("operator", "lt_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let extern_local = func.local(ValType::EXTERNREF)?;
            let float2_local = func.local(ValType::F64)?;
            let float1_local = func.local(ValType::F64)?;
            wasm![
                LocalSet(float2_local),
                LocalTee(extern_local),
                Call(string2float),
                LocalTee(float1_local),
                LocalGet(float1_local),
                F64Ne, // test if float_local is NaN
                LocalGet(float2_local),
                @isnan(t2),
                I32Or,
                If(BlockType::Result(ValType::I32)),
                LocalGet(extern_local),
                LocalGet(float2_local),
                Call(float2string),
                Call(string_lt),
                Else,
                LocalGet(float1_local),
                LocalGet(float2_local),
                F64Lt,
                End,
            ]
        } else if IrType::String.contains(t2) {
            // TODO: try converting to numbers first
            let string_lt = func.registries().external_functions().register(
                ("operator", "lt_string".into()),
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

crate::instructions_test! {tests; operator_lt; t1, t2 ;}
