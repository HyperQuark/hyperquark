use super::super::prelude::*;

pub fn wasm(_func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<Instruction<'static>>> {
    Ok(wasm![I32Eqz])
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::Boolean])
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    Ok(Some(IrType::Boolean))
}

crate::instructions_test! {tests; operator_not; t ;}
