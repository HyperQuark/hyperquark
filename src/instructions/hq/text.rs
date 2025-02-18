use crate::ir::Type as IrType;
use crate::prelude::*;
use crate::wasm::StepFunc;
use wasm_encoder::{Instruction, RefType};

#[derive(Clone, Debug)]
pub struct Fields(pub Box<str>);

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<Instruction<'static>>> {
    let string_idx = func
        .registries()
        .strings()
        .register_default(fields.0.clone())?;
    Ok(vec![
        Instruction::I32Const(string_idx),
        Instruction::TableGet(
            func.registries()
                .tables()
                .register("strings".into(), (RefType::EXTERNREF, 0))?,
        ),
    ])
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([])
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields(val): &Fields) -> HQResult<Option<IrType>> {
    Ok(Some(match &**val {
        bool if bool.to_lowercase() == "true" || bool.to_lowercase() == "false" => {
            IrType::StringBoolean
        }
        num if num.parse::<f64>().is_ok() => {
            if num.parse::<f64>().unwrap().is_nan() {
                IrType::StringNan
            } else {
                IrType::StringNumber
            }
        }
        _ => IrType::StringNan,
    }))
}

crate::instructions_test! {tests; hq_text; @ super::Fields("hello, world!".into())}
