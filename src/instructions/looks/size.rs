use wasm_encoder::MemArg;

use super::super::prelude::*;
use crate::wasm::{StepTarget, mem_layout};

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let ir_target_index: i32 = func
        .target_index()
        .try_into()
        .map_err(|_| make_hq_bug!("target index out of bounds"))?;
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        hq_bad_proj!("looks_size called in stage")
    };
    let local_index = func.local(ValType::F64)?;
    Ok(wasm![
        I32Const(0),
        F64Load(MemArg {
            offset: (mem_layout::stage::BLOCK_SIZE
                + wasm_target_index * mem_layout::sprite::BLOCK_SIZE
                + mem_layout::sprite::SIZE)
                .into(),
            align: 2,
            memory_index: 0,
        }),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::Singleton(IrType::FloatPos))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; looks_size; ; }
