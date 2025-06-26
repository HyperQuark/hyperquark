use crate::wasm::TableOptions;

use super::super::prelude::*;
use wasm_encoder::RefType;

#[derive(Clone, Debug)]
pub struct Fields(pub Box<str>);

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "value": {:?}
    }}"#,
            self.0
        )
    }
}

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
        // string imports always come before any other global, so we don't need to use #LazyGlobalGet
        GlobalGet(string_idx),
    ])
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
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
