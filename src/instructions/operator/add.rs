use crate::ir::types::Type as IrType;
use crate::prelude::*;
use crate::wasm::StepFunc;
use wasm_encoder::{Instruction, ValType};

pub fn wasm(func: &StepFunc, t1: IrType, t2: IrType) -> HQResult<Vec<Instruction<'static>>> {
    Ok(if IrType::QuasiInt.contains(t1) {
        if IrType::QuasiInt.contains(t2) {
            vec![Instruction::I64Add]
        } else if IrType::Float.contains(t2) {
            let f64_local = func.get_local(ValType::F64, 1)?;
            vec![
                Instruction::LocalSet(f64_local),
                Instruction::F64ConvertI64S,
                Instruction::LocalGet(f64_local),
                Instruction::F64Add,
            ]
        } else {
            hq_todo!()
        }
    } else if IrType::Float.contains(t1) {
        if IrType::Float.contains(t2) {
            vec![Instruction::F64Add]
        } else if IrType::QuasiInt.contains(t2) {
            vec![Instruction::F64ConvertI64S, Instruction::F64Add]
        } else {
            hq_todo!();
        }
    } else {
        hq_todo!()
    })
}

#[allow(non_upper_case_globals)]
pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::Number, IrType::Number])
}

// TODO: nan
pub fn output_type(t1: IrType, t2: IrType) -> HQResult<IrType> {
    Ok(if IrType::QuasiInt.contains(t1.or(t2)) {
        IrType::QuasiInt
    } else if (IrType::QuasiInt.contains(t1) && IrType::Float.contains(t2))
        || (IrType::QuasiInt.contains(t2) && IrType::Float.contains(t1))
        || IrType::Float.contains(t1.or(t2))
    {
        IrType::Float
    } else {
        hq_todo!() //IrType::Number
    })
}

crate::instructions_test!(t1, t2);
