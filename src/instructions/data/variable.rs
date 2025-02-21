use super::super::prelude::*;
use crate::ir::RcVar;

#[derive(Debug, Clone)]
pub struct Fields(pub RcVar);

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields(variable): &Fields,
) -> HQResult<Vec<Instruction<'static>>> {
    let global_index: u32 = func
        .registries()
        .variables()
        .register(RcVar::clone(variable))?;
    Ok(wasm![GlobalGet(global_index)])
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([])
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    Ok(Some(IrType::Any))
}

crate::instructions_test!(
    tests;
    data_variable;
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::Any,
                    crate::sb3::VarVal::Float(0.0)
                )
            )
        )
    )
);
