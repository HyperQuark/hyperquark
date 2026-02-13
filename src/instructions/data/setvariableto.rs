use super::super::prelude::*;
use crate::ir::RcVar;
use crate::wasm::WasmProject;

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
    Ok(if let Some(monitor) = var.borrow().monitor().as_ref()
        && *monitor.is_ever_visible.borrow()
    {
        let wasm_input_ty = WasmProject::ir_type_to_wasm(t1)?;
        let local = func.local(wasm_input_ty)?;
        let update_func = func.registries().external_functions().register(
            (
                "data",
                match t1.base_type() {
                    Some(IrType::Boolean) => "update_var_val_bool",
                    Some(IrType::String) => "update_var_val_string",
                    Some(IrType::Int) => "update_var_val_int",
                    Some(IrType::Float) => "update_var_val_float",
                    _ => hq_bug!("bad input type for variable with monitor"),
                }
                .into(),
            ),
            (vec![wasm_input_ty, ValType::EXTERNREF], vec![]),
        )?;
        let variable_string = func
            .registries()
            .strings()
            .register_default(monitor.id.clone())?;
        wasm![
            LocalTee(local),
            GlobalGet(variable_string),
            Call(update_func),
            LocalGet(local),
        ]
    } else {
        wasm![]
    }
    .into_iter()
    .chain(if *local_write.try_borrow()? {
        let local_index: u32 = func.local_variable(&*var.try_borrow()?)?;
        if var.borrow().possible_types().is_base_type() {
            wasm![LocalSet(local_index)]
        } else {
            wasm![
                @boxed(t1),
                LocalSet(local_index)
            ]
        }
    } else {
        let global_index: u32 = func
            .registries()
            .variables()
            .register(&*var.try_borrow()?)?;
        if var.try_borrow()?.possible_types().is_base_type() {
            wasm![#LazyGlobalSet(global_index)]
        } else {
            wasm![
                @boxed(t1),
                #LazyGlobalSet(global_index),
            ]
        }
    })
    .collect())
}

pub fn acceptable_inputs(Fields { var, .. }: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([
        if var.try_borrow()?.possible_types().is_none() {
            IrType::Any
        } else {
            *var.try_borrow()?.possible_types()
        },
    ]))
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub fn const_fold(
    inputs: &[ConstFoldItem],
    state: &mut ConstFoldState,
    Fields { var, .. }: &Fields,
) -> HQResult<ConstFold> {
    if state.vars.contains_key(var.borrow().id()) {
        // if this variable has already been written to, we don't want to overwrite it with some constant
        // value, so explicitly set it as unknown.
        state.vars.insert(
            var.borrow().id().into(),
            ConstFoldItem::Unknown {
                possible_types: *var.borrow().possible_types(),
                opcodes: Rc::from([]),
            },
        );
    } else {
        state
            .vars
            .insert(var.borrow().id().into(), inputs[0].clone());
    }

    Ok(NotFoldable)
}

crate::instructions_test!(
    any_global;
    data_setvariableto;
    t @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::Any,
                    crate::sb3::VarVal::Float(0.0),
                    None,
            ).unwrap()),
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
                    None,
            ).unwrap()),
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
                    None,
            ).unwrap()),
        local_write: RefCell::new(false)
    }
);

crate::instructions_test!(
    int_global;
    data_setvariableto;
    t @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::Int,
                    crate::sb3::VarVal::Int(1),
                    None,
            ).unwrap()),
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
                    None,
            ).unwrap()),
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
                    None,
            ).unwrap()),
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
                    None,
            ).unwrap()),
        local_write: RefCell::new(true)
    }
);

crate::instructions_test!(
    int_local;
    data_setvariableto;
    t @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::Int,
                    crate::sb3::VarVal::Int(1),
                    None,
            ).unwrap()),
        local_write: RefCell::new(true)
    }
);
