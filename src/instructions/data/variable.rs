use super::super::prelude::*;
use crate::ir::RcVar;

#[derive(Debug, Clone)]
pub struct Fields {
    pub var: RefCell<RcVar>,
    pub local_read: RefCell<bool>,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "variable": {},
        "local_read": {}
    }}"#,
            self.var.borrow(),
            self.local_read.borrow()
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields { var, local_read }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    if *local_read.try_borrow()? {
        let local_index: u32 = func.local_variable(&*var.try_borrow()?)?;
        Ok(wasm![LocalGet(local_index)])
    } else {
        let global_index: u32 = func
            .registries()
            .variables()
            .register(&*var.try_borrow()?)?;
        Ok(wasm![GlobalGet(global_index)])
    }
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::new([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields { var, .. }: &Fields) -> HQResult<Option<IrType>> {
    Ok(Some(if var.borrow().possible_types().is_none() {
        IrType::Any
    } else {
        *var.borrow().possible_types()
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

// crate::instructions_test!(
//     any_global;
//     data_variable;
//     @ super::Fields {
//         var: RefCell::new(super::RcVar(
//             Rc::new(
//                 crate::ir::Variable::new(
//                     IrType::Any,
//                     crate::sb3::VarVal::Float(0.0),

//                 )
//             )
//         )),
//         local_read: RefCell::new(false)
//     }
// );

// crate::instructions_test!(
//     float_global;
//     data_variable;
//     @ super::Fields {
//         var: RefCell::new(super::RcVar(
//             Rc::new(
//                 crate::ir::Variable::new(
//                     IrType::Float,
//                     crate::sb3::VarVal::Float(0.0),

//                 )
//             )
//         )),
//         local_read: RefCell::new(false)
//     }
// );

// crate::instructions_test!(
//     string_global;
//     data_variable;
//     @ super::Fields {
//         var: RefCell::new(super::RcVar(
//             Rc::new(
//                 crate::ir::Variable::new(
//                     IrType::String,
//                     crate::sb3::VarVal::String("".into()),

//                 )
//             )
//         )),
//         local_read: RefCell::new(false)
//     }
// );

// crate::instructions_test!(
//     int_global;
//     data_variable;
//     @ super::Fields {
//         var: RefCell::new(super::RcVar(
//             Rc::new(
//                 crate::ir::Variable::new(
//                     IrType::QuasiInt,
//                     crate::sb3::VarVal::Bool(true),

//                 )
//             )
//         )),
//         local_read: RefCell::new(false)
//     }
// );

// crate::instructions_test!(
//     any_local;
//     data_variable;
//     @ super::Fields {
//         var: RefCell::new(super::RcVar(
//             Rc::new(
//                 crate::ir::Variable::new(
//                     IrType::Any,
//                     crate::sb3::VarVal::Float(0.0),

//                 )
//             )
//         )),
//         local_read: RefCell::new(true)
//     }
// );

// crate::instructions_test!(
//     float_local;
//     data_variable;
//     @ super::Fields {
//         var: RefCell::new(super::RcVar(
//             Rc::new(
//                 crate::ir::Variable::new(
//                     IrType::Float,
//                     crate::sb3::VarVal::Float(0.0),

//                 )
//             )
//         )),
//         local_read: RefCell::new(true)
//     }
// );

// crate::instructions_test!(
//     string_local;
//     data_variable;
//     @ super::Fields {
//         var: RefCell::new(super::RcVar(
//             Rc::new(
//                 crate::ir::Variable::new(
//                     IrType::String,
//                     crate::sb3::VarVal::String("".into()),

//                 )
//             )
//         )),
//         local_read: RefCell::new(true)
//     }
// );

// crate::instructions_test!(
//     int_local;
//     data_variable;
//     @ super::Fields {
//         var: RefCell::new(super::RcVar(
//             Rc::new(
//                 crate::ir::Variable::new(
//                     IrType::QuasiInt,
//                     crate::sb3::VarVal::Bool(true),

//                 )
//             )
//         )),
//         local_read: RefCell::new(true)
//     }
// );
