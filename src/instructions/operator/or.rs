use super::super::prelude::*;

pub fn wasm(_func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    Ok(wasm![I32Or])
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::Boolean, IrType::Boolean])
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    Ok(Some(IrType::Boolean))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_or; t1, t2 ;}
