#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "Fields should be passed by reference for type signature consistency"
)]

use super::super::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Fields(pub f64);

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "value": {}
    }}"#,
            self.0
        )
    }
}

pub fn wasm(
    _func: &StepFunc,
    _inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    Ok(wasm![F64Const(fields.0)])
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, &Fields(val): &Fields) -> HQResult<ReturnType> {
    Ok(Singleton(match val {
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
