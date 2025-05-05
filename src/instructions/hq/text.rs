use crate::wasm::TableOptions;

use super::super::prelude::*;
use wasm_encoder::RefType;

#[derive(Clone, Debug)]
pub struct Fields(pub Box<str>);

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let string_idx = func
        .registries()
        .strings()
        .register_default(fields.0.clone())?;
    Ok(wasm![
        I32Const(string_idx),
        TableGet(func.registries().tables().register(
            "strings".into(),
            TableOptions {
                element_type: RefType::EXTERNREF,
                min: 0,
                max: None,
                // this default gets fixed up in src/wasm/tables.rs
                init: None,
            }
        )?,),
    ])
}

pub fn acceptable_inputs(_fields: &Fields) -> Rc<[IrType]> {
    Rc::new([])
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields(val): &Fields) -> HQResult<Option<IrType>> {
    Ok(Some(match &**val {
        bool if bool.to_lowercase() == "true" || bool.to_lowercase() == "false" => {
            IrType::StringBoolean
        }
        num if let Ok(float) = num.parse::<f64>()
            && !float.is_nan() =>
        {
            IrType::StringNumber
        }
        _ => IrType::StringNan,
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests; hq_text; @ super::Fields("hello, world!".into())}
