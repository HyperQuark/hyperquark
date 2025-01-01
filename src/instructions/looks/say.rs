use crate::instructions::{file_block_category, file_block_name};
use crate::ir::types::Type as IrType;
use crate::prelude::*;
use crate::wasm::StepFunc;
use wasm_encoder::{Instruction, ValType};

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<Instruction<'static>>> {
    Ok(if IrType::QuasiInt.contains(inputs[0]) {
      let func_index = func.external_functions().function_index(
          "looks",
          "say_int",
          vec![ValType::I64],
          vec![],
      )?;
      vec![Instruction::Call(func_index)]
    } else if IrType::Float.contains(inputs[0]) {
        let func_index = func.external_functions().function_index(
          "looks",
          "say_float",
          vec![ValType::F64],
          vec![],
      )?;
      vec![Instruction::Call(func_index)]
    } else {
        hq_todo!()
    })
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::Any])
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    if !(IrType::QuasiInt.contains(inputs[0]) || IrType::Float.contains(inputs[0])) {
        hq_todo!()
    }
    Ok(None)
}

crate::instructions_test!(t);
