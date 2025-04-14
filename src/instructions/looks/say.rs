use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    Ok(if IrType::QuasiInt.contains(inputs[0]) {
        let func_index = func
            .registries()
            .external_functions()
            .register(("looks", "say_int"), (vec![ValType::I32], vec![]))?;
        wasm![Call(func_index)]
    } else if IrType::Float.contains(inputs[0]) {
        let func_index = func
            .registries()
            .external_functions()
            .register(("looks", "say_float"), (vec![ValType::F64], vec![]))?;
        wasm![Call(func_index)]
    } else if IrType::String.contains(inputs[0]) {
        let func_index = func
            .registries()
            .external_functions()
            .register(("looks", "say_string"), (vec![ValType::EXTERNREF], vec![]))?;
        wasm![Call(func_index)]
    } else {
        hq_bug!("bad input")
    })
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::String.or(IrType::Number)])
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<Option<IrType>> {
    if !(IrType::Number.or(IrType::String).contains(inputs[0])) {
        hq_todo!("unimplemented input type: {:?}", inputs)
    }
    Ok(None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = true;

crate::instructions_test! {tests; looks_say; t}
