use wasm_encoder::ConstExpr;

use super::super::prelude::*;
use crate::wasm::{GlobalExportable, GlobalMutable};

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let global_index = func.registries().globals().register(
        "sensing_timer".into(),
        (
            ValType::F64,
            ConstExpr::f64_const(0.0.into()),
            GlobalMutable(true),
            GlobalExportable(true),
        ),
    )?;
    Ok(wasm![
        F64Const(0.0.into()),
        #LazyGlobalSet(global_index),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

crate::instructions_test! {tests; sensing_reset_timer; ;}
