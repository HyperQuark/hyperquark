use super::super::prelude::*;

#[derive(Clone, Debug)]
pub struct Fields {
    pub debug: bool,
    pub target_idx: u32,
}

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    &Fields { debug, target_idx }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let prefix = String::from(if debug { "say_debug" } else { "say" });
    let itarget_idx: i32 = target_idx
        .try_into()
        .map_err(|_| make_hq_bug!("target index out of bounds"))?;
    Ok(if IrType::QuasiInt.contains(inputs[0]) {
        let func_index = func.registries().external_functions().register(
            ("looks", format!("{prefix}_int").into_boxed_str()),
            (vec![ValType::I32, ValType::I32], vec![]),
        )?;
        wasm![I32Const(itarget_idx), Call(func_index)]
    } else if IrType::Float.contains(inputs[0]) {
        let func_index = func.registries().external_functions().register(
            ("looks", format!("{prefix}_float").into_boxed_str()),
            (vec![ValType::F64, ValType::I32], vec![]),
        )?;
        wasm![I32Const(itarget_idx), Call(func_index)]
    } else if IrType::String.contains(inputs[0]) {
        let func_index = func.registries().external_functions().register(
            ("looks", format!("{prefix}_string").into_boxed_str()),
            (vec![ValType::EXTERNREF, ValType::I32], vec![]),
        )?;
        wasm![I32Const(itarget_idx), Call(func_index)]
    } else {
        hq_bug!("bad input")
    })
}

pub fn acceptable_inputs(_fields: &Fields) -> Rc<[IrType]> {
    Rc::new([IrType::String.or(IrType::Number)])
}

pub fn output_type(inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    if !(IrType::Number.or(IrType::String).contains(inputs[0])) {
        hq_todo!("unimplemented input type: {:?}", inputs)
    }
    Ok(None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = true;

crate::instructions_test! {tests_debug; looks_say; t @ super::Fields { debug: true, target_idx: 0 }}
crate::instructions_test! {tests_non_debug; looks_say; t @ super::Fields { debug: false, target_idx: 0, }}
