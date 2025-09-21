#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "Fields should be passed by reference for type signature consistency"
)]

use super::super::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Fields {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "r": {},
        "g": {},
        "b": {}
    }}"#,
            self.r, self.g, self.b
        )
    }
}

pub fn wasm(
    _func: &StepFunc,
    _inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    Ok(wasm![I32Const(
        i32::from(fields.r) << 16 | i32::from(fields.g) << 8 | i32::from(fields.b)
    )])
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    Ok(Singleton(IrType::ColorRGB))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; hq_color_rgb; @ super::Fields { r: 134, g: 56, b: 109 }}
