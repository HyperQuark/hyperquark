#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "Fields should be passed by reference for type signature consistency"
)]

use super::super::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Fields(pub i32);

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
    Ok(wasm![I32Const(fields.0)])
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, &Fields(val): &Fields) -> HQResult<Option<IrType>> {
    Ok(Some(match val {
        0 => IrType::IntZero,
        pos if pos > 0 => IrType::IntPos,
        neg if neg < 0 => IrType::IntNeg,
        _ => unreachable!(),
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; hq_integer; @ super::Fields(0)}
