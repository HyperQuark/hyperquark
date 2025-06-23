use super::super::prelude::*;
use crate::ir::RcVar;

/// we need these fields to be mutable for optimisations to be feasible
#[derive(Debug, Clone)]
pub struct Fields {
    pub var: RefCell<RcVar>,
    pub local_write: RefCell<bool>,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "variable": {},
        "local_write": {}
    }}"#,
            self.var.borrow(),
            self.local_write.borrow()
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    Fields { var, local_write }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let t1 = inputs[0];
    if *local_write.try_borrow()? {
        let local_index: u32 = func.local_variable(&*var.try_borrow()?)?;
        if var.borrow().possible_types().is_base_type() {
            Ok(wasm![LocalSet(local_index)])
        } else {
            Ok(wasm![
                @boxed(t1),
                LocalSet(local_index)
            ])
        }
    } else {
        let global_index: u32 = func
            .registries()
            .variables()
            .register(&*var.try_borrow()?)?;
        if var.try_borrow()?.possible_types().is_base_type() {
            Ok(wasm![GlobalSet(global_index)])
        } else {
            Ok(wasm![
                @boxed(t1),
                GlobalSet(global_index),
            ])
        }
    }
}

pub fn acceptable_inputs(Fields { var, .. }: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([if var.try_borrow()?.possible_types().is_none() {
        IrType::Any
    } else {
        *var.try_borrow()?.possible_types()
    }]))
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    Ok(None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test!(
    any_global;
    data_setvariableto;
    t @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::Any,
                    crate::sb3::VarVal::Float(0.0),


            )),
        local_write: RefCell::new(false)
    }
);

crate::instructions_test!(
    float_global;
    data_setvariableto;
    t @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::Float,
                    crate::sb3::VarVal::Float(0.0),


            )),
        local_write: RefCell::new(false)
    }
);

crate::instructions_test!(
    string_global;
    data_setvariableto;
    t @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::String,
                    crate::sb3::VarVal::String("".into()),


            )),
        local_write: RefCell::new(false)
    }
);

crate::instructions_test!(
    int_global;
    data_setvariableto;
    t @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::QuasiInt,
                    crate::sb3::VarVal::Bool(true),


            )),
        local_write: RefCell::new(false)
    }
);

crate::instructions_test!(
    any_local;
    data_setvariableto;
    t @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::Any,
                    crate::sb3::VarVal::Float(0.0),


            )),
        local_write: RefCell::new(true)
    }
);

crate::instructions_test!(
    float_local;
    data_setvariableto;
    t @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::Float,
                    crate::sb3::VarVal::Float(0.0),


            )),
        local_write: RefCell::new(true)
    }
);

crate::instructions_test!(
    string_local;
    data_setvariableto;
    t @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::String,
                    crate::sb3::VarVal::String("".into()),


            )),
        local_write: RefCell::new(true)
    }
);

crate::instructions_test!(
    int_local;
    data_setvariableto;
    t @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::QuasiInt,
                    crate::sb3::VarVal::Bool(true),


            )),
        local_write: RefCell::new(true)
    }
);
