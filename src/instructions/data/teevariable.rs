/// this is currently just a convenience block. if we get thread-scoped variables
/// (i.e. locals) then this can actually use the wasm tee instruction.
use super::super::prelude::*;
use crate::ir::RcVar;
use crate::wasm::WasmProject;

#[derive(Debug, Clone)]
pub struct Fields {
    pub var: RefCell<RcVar>,
    pub local_read_write: RefCell<bool>,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "variable": {},
        "local_read_write": {}
    }}"#,
            self.var.borrow(),
            self.local_read_write.borrow()
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    Fields {
        var,
        local_read_write,
    }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let t1 = inputs[0];
    Ok(if let Some(monitor) = var.borrow().monitor().as_ref()
        && *monitor.is_ever_visible.borrow()
    {
        let wasm_input_ty = WasmProject::ir_type_to_wasm(t1);
        let local = func.local(wasm_input_ty)?;
        func.free_local(local)?;
        let update_func = func.registries().external_functions().register(
            (
                "data",
                match t1.base_type() {
                    Some(IrType::Boolean) => "update_var_val_boolean",
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
    .chain(if *local_read_write.try_borrow()? {
        let local_index: u32 = func.local_variable(&*var.try_borrow()?)?;
        if var.borrow().possible_types().is_base_type() {
            wasm![LocalTee(local_index)]
        } else {
            wasm![
                @boxed(t1),
                LocalTee(local_index)
            ]
        }
    } else {
        let global_index: u32 = func
            .registries()
            .variables()
            .register(&*var.try_borrow()?)?;
        if var.borrow().possible_types().is_base_type() {
            wasm![#LazyGlobalSet(global_index), #LazyGlobalGet(global_index)]
        } else {
            wasm![
                @boxed(t1),
                #LazyGlobalSet(global_index),
                #LazyGlobalGet(global_index)
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

pub fn output_type(_inputs: Rc<[IrType]>, Fields { var, .. }: &Fields) -> HQResult<ReturnType> {
    Ok(Singleton(*var.try_borrow()?.possible_types()))
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

#[cfg(test)]
mod test {
    use super::super::variable::test_util::*;
    use super::*;
    use crate::instructions::tests::assert_valid_json;
    use crate::wasm::WasmFlags;
    use crate::wasm::flags::unit_test_wasm_features;

    #[test]
    fn fields_display_is_valid_json() {
        let fields = make_fields(
            IrType::Any,
            crate::sb3::VarVal::Float(0.0),
            true,
            WasmFlags::new(unit_test_wasm_features()),
        );
        assert_valid_json(format!("{fields}"));
    }

    pub fn make_fields(
        ty: IrType,
        initial: VarVal,
        local_read_write: bool,
        flags: WasmFlags,
    ) -> Fields {
        Fields {
            var: make_var(ty, initial, None, flags),
            local_read_write: RefCell::new(local_read_write),
        }
    }
}

crate::instructions_test!(
    mod test_any_global for data_teevariable(t) {
        fields = super::test::make_fields(IrType::Any, crate::sb3::VarVal::Float(0.0), false, flags());
    }
);

crate::instructions_test!(
    mod test_float_global for data_teevariable(t) {
        fields = super::test::make_fields(IrType::Float, crate::sb3::VarVal::Float(0.0), false, flags());
    }
);

crate::instructions_test!(
    mod test_string_global for data_teevariable(t) {
        fields = super::test::make_fields(IrType::String, crate::sb3::VarVal::String("".into()), false, flags());
    }
);

crate::instructions_test!(
    mod test_int_global for data_teevariable(t) {
        fields = super::test::make_fields(IrType::Int, crate::sb3::VarVal::Int(0), false, flags());
    }
);

crate::instructions_test!(
    mod test_any_local for data_teevariable(t) {
        fields = super::test::make_fields(IrType::Any, crate::sb3::VarVal::Float(0.0), true, flags());
    }
);

crate::instructions_test!(
    mod test_float_local for data_teevariable(t) {
        fields = super::test::make_fields(IrType::Float, crate::sb3::VarVal::Float(0.0), true, flags());
    }
);

crate::instructions_test!(
    mod test_string_local for data_teevariable(t) {
        fields = super::test::make_fields(IrType::String, crate::sb3::VarVal::String("".into()), true, flags());
    }
);

crate::instructions_test!(
    mod test_int_local for data_teevariable(t) {
        fields = super::test::make_fields(IrType::Int, crate::sb3::VarVal::Int(0), true, flags());
    }
);
