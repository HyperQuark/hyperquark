use super::super::prelude::*;
use crate::ir::RcVar;

#[derive(Debug, Clone)]
pub struct Fields(pub RcVar);

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    Fields(variable): &Fields,
) -> HQResult<Vec<Instruction<'static>>> {
    let global_index: u32 = func
        .registries()
        .variables()
        .register(RcVar::clone(variable))?;
    let t1 = inputs[0];
    Ok(wasm![
        @boxed(t1),
        GlobalSet(global_index)
    ])
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::Any])
    //Rc::new([IrType::String.or(IrType::Number)])
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    Ok(None)
}

crate::instructions_test!(
    tests;
    data_setvariableto;
    t
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
