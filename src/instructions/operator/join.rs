use crate::ir::Type as IrType;
use crate::prelude::*;
use crate::wasm::StepFunc;
use wasm_encoder::{Instruction, ValType};

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<Instruction<'static>>> {
    hq_assert_eq!(inputs.len(), 2);
    let func_index = func.registries().external_functions().register(
        ("operator", "join"),
        (
            vec![ValType::EXTERNREF, ValType::EXTERNREF],
            vec![ValType::EXTERNREF],
        ),
    )?;
    Ok(vec![Instruction::Call(func_index)])
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::String, IrType::String])
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    Ok(Some(IrType::String))
}

crate::instructions_test! {tests; t1, t2 ;}
