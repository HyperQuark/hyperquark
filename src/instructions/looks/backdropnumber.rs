use wasm_encoder::MemArg;

use super::super::prelude::*;
use crate::wasm::mem_layout;

pub fn wasm(_func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let offset = mem_layout::stage::COSTUME;

    Ok(wasm![
        I32Const(0),
        I32Load(MemArg {
            offset: offset.into(),
            align: 2,
            memory_index: 0,
        }),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::Singleton(IrType::IntPos))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

crate::instructions_test! {tests; looks_backdropnumber; ; }
