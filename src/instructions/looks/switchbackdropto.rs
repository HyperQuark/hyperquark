use wasm_encoder::MemArg;

use super::super::prelude::*;
use crate::wasm::mem_layout;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let func_index = func.registries().external_functions().register(
        ("looks", "switchbackdropto".into()),
        (vec![ValType::I32], vec![]),
    )?;
    let offset = mem_layout::stage::COSTUME;

    let local_index = func.local(ValType::I32)?;
    Ok(if IrType::QuasiInt.contains(inputs[0]) {
        wasm![
            LocalSet(local_index),
            I32Const(0),
            LocalGet(local_index),
            I32Store(MemArg {
                offset: offset.into(),
                align: 2,
                memory_index: 0,
            }),
            LocalGet(local_index),
            Call(func_index),
        ]
    } else {
        hq_todo!("non-integer input types for looks_switchbackdropto")
    })
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    // TODO: accept non-integer values (try to find costume name)
    Ok(Rc::from([IrType::Int]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = true;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

crate::instructions_test! {tests; looks_switchbackdropto; t ; }
