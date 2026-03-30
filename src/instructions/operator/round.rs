use super::super::prelude::*;

pub fn wasm(func: &StepFunc, inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    hq_assert_eq!(inputs.len(), 1);
    let t1 = inputs[0];
    Ok(if IrType::QuasiInt.contains(t1) {
        wasm![]
    } else if IrType::Float.contains(t1) {
        wasm![
            @nanreduce(t1),
            F64Nearest,
        ]
    } else {
        hq_bug!("bad input: {:?}", inputs)
    })
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Number]))
}

pub fn output_type(inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    hq_assert_eq!(inputs.len(), 1);
    let t1 = inputs[0];
    Ok(Singleton(if IrType::QuasiInt.contains(t1) {
        t1
    } else {
        IrType::Float
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

crate::instructions_test! {tests; operator_round; t }
