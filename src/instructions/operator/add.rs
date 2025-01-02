use crate::ir::Type as IrType;
use crate::prelude::*;
use crate::wasm::StepFunc;
use wasm_encoder::{Instruction, ValType};

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<Instruction<'static>>> {
    let t1 = inputs[0];
    let t2 = inputs[1];
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
        hq_todo!() //IrType::Number
    }))
}

#[cfg(test)]
mod tests {
    crate::instructions_test!{test; t1, t2}
}
