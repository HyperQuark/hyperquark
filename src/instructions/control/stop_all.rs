use wasm_encoder::{ConstExpr, HeapType};

use super::super::prelude::*;
use crate::wasm::{GlobalExportable, GlobalMutable, ThreadsTable};

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let threads_count = func.registries().globals().register(
        "threads_count".into(),
        (
            ValType::I32,
            ConstExpr::i32_const(0),
            GlobalMutable(true),
            GlobalExportable(true),
        ),
    )?;

    let threads_table = func.registries().tables().register::<ThreadsTable, _>()?;
    let thread_struct_type = func.registries().types().thread_struct_type()?;

    Ok(wasm![
        I32Const(0),
        #LazyGlobalSet(threads_count),
        I32Const(0),
        RefNull(HeapType::Concrete(thread_struct_type)),
        TableSize(threads_table),
        TableFill(threads_table),
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
