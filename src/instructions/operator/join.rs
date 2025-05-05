use wasm_encoder::HeapType;

use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let func_index = func.registries().external_functions().register(
        ("wasm:js-string", "concat".into()),
        (
            vec![ValType::EXTERNREF, ValType::EXTERNREF],
            vec![ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::EXTERN,
            })],
        ),
    )?;
    Ok(wasm![Call(func_index)])
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::String, IrType::String])
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    Ok(Some(IrType::String))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_join; t1, t2 ;}
