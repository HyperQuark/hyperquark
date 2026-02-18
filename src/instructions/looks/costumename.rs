use wasm_encoder::MemArg;

use super::super::prelude::*;
use crate::wasm::registries::tables::TableOptions;
use crate::wasm::{StepTarget, mem_layout};

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let wasm_target_index = match func.target() {
        StepTarget::Sprite(index) => index,
        StepTarget::Stage => 0,
    };
    let offset = mem_layout::stage::BLOCK_SIZE
        + wasm_target_index * mem_layout::sprite::BLOCK_SIZE
        + mem_layout::sprite::COSTUME;

    let costume_names = func
        .costume_names()
        .get(func.target_index() as usize)
        .ok_or_else(|| make_hq_bug!("target index out of bounds in costume names vec"))?;

    let costume_name_table = func.registries().tables().register_dyn(
        format!("costume_names_{}", func.target_index()).into_boxed_str(),
        TableOptions {
            element_type: RefType::EXTERNREF,
            min: costume_names.len() as u64,
            max: Some(costume_names.len() as u64),
            init: None,
            export_name: None,
        },
    )?;

    for costume_name in costume_names {
        // make sure that strings are all registered so that the element segment can refer to globals that actually exist
        func.registries()
            .strings()
            .register_default::<usize>(costume_name.clone())?;
    }

    Ok(wasm![
        I32Const(0),
        I32Load(MemArg {
            offset: offset.into(),
            align: 2,
            memory_index: 0,
        }),
        TableGet(costume_name_table),
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::Singleton(IrType::String))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

// crate::instructions_test! {tests; looks_costumename; ; }
