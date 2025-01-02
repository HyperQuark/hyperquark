use crate::ir::Type as IrType;
use crate::prelude::*;
use crate::wasm::StepFunc;
use wasm_encoder::{Instruction, ValType};

#[derive(Clone, Copy, Debug)]
pub struct Fields(pub f64);

pub fn wasm(_func: &StepFunc, _inputs: Rc<[IrType]>, fields: &Fields) -> HQResult<Vec<Instruction<'static>>> {
    Ok(vec![Instruction::F64Const(fields.0)])
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([])
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    Ok(Some(IrType::Float))
}

#[cfg(test)]
mod tests {
    use super::Fields;
    crate::instructions_test!{_0;; super::Fields(0.0)}
    crate::instructions_test!{_1;; super::Fields(1.0)}
}