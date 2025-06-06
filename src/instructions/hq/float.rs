#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "Fields should be passed by reference for type signature consistency"
)]

use super::super::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Fields(pub f64);

pub fn wasm(
    _func: &StepFunc,
    _inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    Ok(wasm![F64Const(fields.0)])
}

pub fn acceptable_inputs(_fields: &Fields) -> Rc<[IrType]> {
    Rc::new([])
}

pub fn output_type(_inputs: Rc<[IrType]>, &Fields(val): &Fields) -> HQResult<Option<IrType>> {
    Ok(Some(match val {
        0.0 => IrType::FloatZero,
        f64::INFINITY => IrType::FloatPosInf,
        f64::NEG_INFINITY => IrType::FloatNegInf,
        nan if f64::is_nan(nan) => IrType::FloatNan,
        int if int % 1.0 == 0.0 && int > 0.0 => IrType::FloatPosInt,
        int if int % 1.0 == 0.0 && int < 0.0 => IrType::FloatNegInt,
        frac if frac > 0.0 => IrType::FloatPosFrac,
        frac if frac < 0.0 => IrType::FloatNegFrac,
        _ => unreachable!(),
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; hq_float; @ super::Fields(0.0)}
