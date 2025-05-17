use wasm_encoder::HeapType;

use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 2);
    let func_index = func.registries().external_functions().register(
        ("wasm:js-string", "substring".into()),
        (
            vec![ValType::EXTERNREF, ValType::I32, ValType::I32],
            vec![ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::EXTERN,
            })],
        ),
    )?;
    let i32_local = func.local(ValType::I32)?;
    let ef_local = func.local(ValType::EXTERNREF)?;
    Ok(wasm![
        LocalSet(ef_local),
        LocalSet(i32_local),
        LocalGet(ef_local),
        LocalGet(i32_local),
        I32Const(1),
        I32Sub,
        LocalGet(i32_local),
        Call(func_index),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::new([IrType::Int, IrType::String]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    Ok(Some(IrType::String))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; operator_letter_of; t1, t2 ;}
