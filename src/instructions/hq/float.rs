use crate::ir::Type as IrType;
use crate::prelude::*;
use crate::wasm::StepFunc;
use wasm_encoder::Instruction;

#[derive(Clone, Copy, Debug)]
pub struct Fields(pub f64);

pub fn wasm(
    _func: &StepFunc,
    _inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<Instruction<'static>>> {
    Ok(vec![Instruction::F64Const(fields.0)])
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
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

crate::instructions_test! {tests;; super::Fields(0.0)}
