use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let func_index = func.registries().external_functions().register(
        ("operator", "contains".into()),
        (
            vec![ValType::EXTERNREF, ValType::EXTERNREF],
            vec![ValType::I32],
        ),
    )?;
    Ok(wasm![Call(func_index)])
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::String, IrType::String])
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    Ok(Some(IrType::Boolean))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_contains; t1, t2 ;}
