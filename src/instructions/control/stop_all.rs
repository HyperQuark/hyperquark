use wasm_encoder::HeapType;

use super::super::prelude::*;
use crate::instructions_test;
use crate::wasm::StepTarget;

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
    let thread_struct_type = func.registries().types().thread_struct_type()?;
    let total_threads_count = func.registries().globals().threads_count()?;
    let num_sprites = func.costume_names().len() as u32;

    Ok(wasm![
        I32Const(0),
        #LazyGlobalSet(total_threads_count),
    ]
    .into_iter()
    .chain(clear_thread(
        func.registries()
            .globals()
            .target_threads_count(StepTarget::Stage)?,
        func.registries()
            .tables()
            .threads_table(StepTarget::Stage, func.registries().types())?,
        thread_struct_type,
    ))
    .chain(
        (0..num_sprites)
            .map(|n| {
                let step_target = StepTarget::Sprite(n);
                Ok(clear_thread(
                    func.registries()
                        .globals()
                        .target_threads_count(step_target)?,
                    func.registries()
                        .tables()
                        .threads_table(step_target, func.registries().types())?,
                    thread_struct_type,
                ))
            })
            .collect::<HQResult<Box<[_]>>>()?
            .into_iter()
            .flatten(),
    )
    .collect())
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
