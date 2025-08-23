#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "Fields should be passed by reference for type signature consistency"
)]

use super::super::prelude::*;

#[derive(Clone, Copy, Debug)]
pub struct Fields(pub bool);

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
    Ok(wasm![I32Const(fields.0.into())])
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, &Fields(val): &Fields) -> HQResult<ReturnType> {
    Ok(Singleton(match val {
        true => IrType::BooleanTrue,
        false => IrType::BooleanFalse,
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests_false; hq_boolean; @ super::Fields(false)} 
crate::instructions_test! {tests_true; hq_boolean; @ super::Fields(true)}
