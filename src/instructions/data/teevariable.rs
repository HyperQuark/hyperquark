/// this is currently just a convenience block. if we get thread-scoped variables
/// (i.e. locals) then this can actually use the wasm tee instruction.
use super::super::prelude::*;
use crate::ir::RcVar;

#[derive(Debug, Clone)]
pub struct Fields(pub RcVar);

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    Fields(variable): &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let global_index: u32 = func
        .registries()
        .variables()
        .register(RcVar::clone(variable))?;
    let t1 = inputs[0];
    if variable.0.possible_types().is_base_type() {
        Ok(wasm![GlobalSet(global_index), GlobalGet(global_index)])
    } else {
        Ok(wasm![
            @boxed(t1),
            GlobalSet(global_index),
            GlobalGet(global_index)
        ])
    }
}

pub fn acceptable_inputs(Fields(rcvar): &Fields) -> Rc<[IrType]> {
    Rc::new([if rcvar.0.possible_types().is_none() {
        IrType::Any
    } else {
        *rcvar.0.possible_types()
    }])
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields(rcvar): &Fields) -> HQResult<Option<IrType>> {
    Ok(Some(*rcvar.0.possible_types()))
}

pub const YIELDS: bool = false;

crate::instructions_test!(
    any;
    data_teevariable;
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

crate::instructions_test!(
    float;
    data_teevariable;
    t
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::Float,
                    crate::sb3::VarVal::Float(0.0)
                )
            )
        )
    )
);

crate::instructions_test!(
    string;
    data_teevariable;
    t
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::String,
                    crate::sb3::VarVal::String("".into())
                )
            )
        )
    )
);

crate::instructions_test!(
    int;
    data_teevariable;
    t
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::QuasiInt,
                    crate::sb3::VarVal::Bool(true)
                )
            )
        )
    )
);
