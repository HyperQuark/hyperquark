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
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
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
                Call(string_gt),
                Else,
                F64ConvertI32S,
                LocalGet(float_local),
                F64Gt,
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
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
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
                Call(string_gt),
                Else,
                LocalGet(float2_local),
                F64Gt,
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
            let string_gt = func.registries().external_functions().register(
                ("operator", "gt_string".into()),
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
                Call(string_gt),
                Else,
                LocalGet(float_local),
                LocalGet(int_local),
                F64ConvertI32S,
                F64Gt,
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
            let string_gt = func.registries().external_functions().register(
                ("operator", "gt_string".into()),
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
                Call(string_gt),
                Else,
                LocalGet(float1_local),
                LocalGet(float2_local),
                F64Gt,
                End,
            ]
        } else if IrType::String.contains(t2) {
            let string2float = func.registries().external_functions().register(
                ("cast", "string2float".into()),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            let string_gt = func.registries().external_functions().register(
                ("operator", "gt_string".into()),
                (
                    vec![ValType::EXTERNREF, ValType::EXTERNREF],
                    vec![ValType::I32],
                ),
            )?;
            let extern1_local = func.local(ValType::EXTERNREF)?;
            let extern2_local = func.local(ValType::EXTERNREF)?;
            let float1_local = func.local(ValType::F64)?;
            let float2_local = func.local(ValType::F64)?;
            wasm![
                LocalTee(extern2_local),
                Call(string2float),
                LocalSet(float2_local),
                LocalTee(extern1_local),
                Call(string2float),
                LocalTee(float1_local),
                LocalGet(float1_local),
                F64Ne,
                LocalGet(float2_local),
                LocalGet(float2_local),
                F64Ne,
                I32Or,
                If(BlockType::Result(ValType::I32)),
                LocalGet(extern1_local),
                LocalGet(extern2_local),
                Call(string_gt),
                Else,
                LocalGet(float1_local),
                LocalGet(float2_local),
                F64Gt,
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

pub fn const_fold(inputs: &[ConstFoldItem], _state: &mut ConstFoldState) -> HQResult<ConstFold> {
    hq_assert!(inputs.len() == 2);
    Ok(
        if let (ConstFoldItem::Basic(val1), ConstFoldItem::Basic(val2)) = (&inputs[0], &inputs[1]) {
            ConstFold::Folded(Rc::from([match (val1, val2) {
                (VarVal::Int(i1), VarVal::Int(i2)) => ConstFoldItem::Basic(VarVal::Bool(i1 > i2)),
                (VarVal::Float(f1), VarVal::Float(f2)) => {
                    ConstFoldItem::Basic(VarVal::Bool(f1 > f2))
                }
                _ => return Ok(NotFoldable),
            }]))
        } else {
            NotFoldable
        },
    )
}

crate::instructions_test! {tests; operator_lt; t1, t2 ;}
