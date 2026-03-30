use wasm_encoder::ConstExpr;

use super::super::prelude::*;
use crate::wasm::{GlobalExportable, GlobalMutable};

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let empty_string_index = func.registries().strings().register_default("".into())?;
    let global_index = func.registries().globals().register(
        "sensing_answer".into(),
        (
            ValType::EXTERNREF,
            ConstExpr::global_get(empty_string_index),
            GlobalMutable(true),
            GlobalExportable(true),
        ),
    )?;
    Ok(wasm![
        #LazyGlobalGet(global_index),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(Singleton(IrType::String))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

crate::instructions_test! {tests; sensing_answer; ;}
