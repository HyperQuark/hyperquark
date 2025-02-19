use crate::ir::Type as IrType;
use crate::prelude::*;
use crate::wasm::StepFunc;
use wasm_encoder::{Instruction, ValType};
use wasm_gen::wasm;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<Instruction<'static>>> {
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
            hq_todo!()
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
            hq_todo!()
        }
    } else {
        hq_todo!()
    })
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::Number, IrType::Number])
}

// TODO: nan
pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    let t1 = inputs[0];
    let t2 = inputs[1];
    Ok(Some(if IrType::QuasiInt.contains(t1.or(t2)) {
        IrType::QuasiInt
    } else if (IrType::QuasiInt.contains(t1) && IrType::Float.contains(t2))
        || (IrType::QuasiInt.contains(t2) && IrType::Float.contains(t1))
        || IrType::Float.contains(t1.or(t2))
    {
        IrType::Float
    } else {
        IrType::Number
    }))
}

crate::instructions_test! {tests; operator_add; t1, t2 ;}
