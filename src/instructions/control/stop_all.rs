use wasm_encoder::HeapType;

use super::super::prelude::*;
use crate::instructions_test;
use crate::wasm::registries::functions::static_functions::DynArrayClear;
use crate::wasm::registries::types::{
    TNullable, TStackArray, TTargetThreadArray, TTargetThreadsStruct, TThreadArray, TType,
};

fn clear_thread(
    threads_count: u32,
    threads_table: u32,
    thread_struct_type: u32,
) -> Vec<InternalInstruction> {
    wasm![
        I32Const(0),
        #LazyGlobalSet(threads_count),
        I32Const(0),
        RefNull(HeapType::Concrete(thread_struct_type)),
        TableSize(threads_table),
        TableFill(threads_table),
    ]
}

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let local_target_counter = func.local(ValType::I32)?;
    func.free_local(local_target_counter)?;
    let threadss_global = func
        .registries()
        .globals()
        .threadss(func.registries().types(), func.costume_names().len() as u32)?;
    let total_threads_count = func.registries().globals().threads_count()?;
    let num_targets = 1 + func.costume_names().len() as i32;
    let array_type = func
        .registries()
        .types()
        .register_comp::<TTargetThreadArray, _>()?;
    let dyn_array_clear = func
        .registries()
        .static_functions()
        .register::<DynArrayClear<TNullable<TStackArray>>, _>()?;

    Ok(wasm![
        I32Const(0),
        #LazyGlobalSet(total_threads_count),
        I32Const(0),
        LocalSet(local_target_counter),
        Loop(wasm_encoder::BlockType::Empty),
        #LazyGlobalGet(threadss_global),
        LocalGet(local_target_counter),
        ArrayGet(array_type),
        StructGet {
            struct_type_index: TTargetThreadsStruct::ty(func.registries().types())?,
            field_index: 1,
        },
        #StaticFunctionCall(dyn_array_clear),
        LocalGet(local_target_counter),
        I32Const(1),
        I32Add,
        LocalTee(local_target_counter),
        I32Const(num_targets),
        I32LtS,
        BrIf(0),
        End,
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

instructions_test!(
    mod test for control_stop_all {}
);
