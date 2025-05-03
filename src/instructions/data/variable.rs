use super::super::prelude::*;
use crate::ir::RcVar;

#[derive(Debug, Clone)]
pub struct Fields(pub RcVar);

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields(variable): &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    if variable.0.local() {
        let local_index: u32 = func.local_variable(variable)?;
        Ok(wasm![LocalGet(local_index)])
    } else {
        let global_index: u32 = func.registries().variables().register(variable)?;
        Ok(wasm![GlobalGet(global_index)])
    }
}

pub fn acceptable_inputs(_fields: &Fields) -> Rc<[IrType]> {
    Rc::new([])
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields(rcvar): &Fields) -> HQResult<Option<IrType>> {
    Ok(Some(if rcvar.0.possible_types().is_none() {
        IrType::Any
    } else {
        *rcvar.0.possible_types()
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test!(
    any_global;
    data_variable;
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::Any,
                    crate::sb3::VarVal::Float(0.0),
                    false,
                )
            )
        )
    )
);

crate::instructions_test!(
    float_global;
    data_variable;
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::Float,
                    crate::sb3::VarVal::Float(0.0),
                    false,
                )
            )
        )
    )
);

crate::instructions_test!(
    string_global;
    data_variable;
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::String,
                    crate::sb3::VarVal::String("".into()),
                    false,
                )
            )
        )
    )
);

crate::instructions_test!(
    int_global;
    data_variable;
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::QuasiInt,
                    crate::sb3::VarVal::Bool(true),
                    false,
                )
            )
        )
    )
);

crate::instructions_test!(
    any_local;
    data_variable;
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::Any,
                    crate::sb3::VarVal::Float(0.0),
                    true,
                )
            )
        )
    )
);

crate::instructions_test!(
    float_local;
    data_variable;
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::Float,
                    crate::sb3::VarVal::Float(0.0),
                    true,
                )
            )
        )
    )
);

crate::instructions_test!(
    string_local;
    data_variable;
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::String,
                    crate::sb3::VarVal::String("".into()),
                    true,
                )
            )
        )
    )
);

crate::instructions_test!(
    int_local;
    data_variable;
    @ super::Fields(
        super::RcVar(
            Rc::new(
                crate::ir::Variable::new(
                    IrType::QuasiInt,
                    crate::sb3::VarVal::Bool(true),
                    true,
                )
            )
        )
    )
);
