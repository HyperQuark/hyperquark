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
        Ok(wasm![#LazyGlobalGet(global_index)])
    }
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields { var, .. }: &Fields) -> HQResult<ReturnType> {
    Ok(Singleton(if var.borrow().possible_types().is_none() {
        IrType::Any
    } else {
        *var.borrow().possible_types()
    }))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub fn const_fold(
    _inputs: &[ConstFoldItem],
    state: &ConstFoldState,
    Fields { var, .. }: &Fields,
) -> HQResult<ConstFold> {
    if let Some(const_val) = state.vars.get(var.borrow().id())
        && !matches!(const_val, ConstFoldItem::Unknown { .. })
    {
        let var_ref = var.try_borrow()?;
        let possible_types = var_ref.possible_types();
        Ok(ConstFold::Folded(
            if possible_types.is_base_type() || matches!(const_val, ConstFoldItem::Boxed { .. }) {
                Rc::from([const_val.clone()])
            } else if let ConstFoldItem::Basic(var_val) = const_val {
                Rc::from([ConstFoldItem::Boxed(var_val.clone(), *possible_types)])
            } else {
                return Ok(NotFoldable);
            },
        ))
    } else {
        Ok(NotFoldable)
    }
}

#[cfg(test)]
pub mod test_util {
    use super::*;
    use crate::ir::{IrMonitor, RcVar};
    use crate::wasm::WasmFlags;

    pub fn make_var(
        ty: IrType,
        initial: crate::sb3::VarVal,
        monitor: Option<IrMonitor>,
        flags: WasmFlags,
    ) -> RefCell<RcVar> {
        RefCell::new(crate::ir::RcVar::new(ty, &initial, monitor, &flags).unwrap())
    }
}

#[cfg(test)]
mod test {
    use test_util::*;

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

    pub fn make_fields(ty: IrType, initial: VarVal, local_read: bool, flags: WasmFlags) -> Fields {
        Fields {
            var: make_var(ty, initial, None, flags),
            local_read: RefCell::new(local_read),
        }
    }
}

crate::instructions_test!(
    mod test_any_global for data_variable {
        fields = super::test::make_fields(IrType::Any, crate::sb3::VarVal::Float(0.0), false, flags());
    }
);

crate::instructions_test!(
    mod test_float_global for data_variable {
        fields = super::test::make_fields(IrType::Float, crate::sb3::VarVal::Float(0.0), false, flags());
    }
);

crate::instructions_test!(
    mod test_string_global for data_variable {
        fields = super::test::make_fields(IrType::String, crate::sb3::VarVal::String("".into()), false, flags());
    }
);

crate::instructions_test!(
    mod test_int_global for data_variable {
        fields = super::test::make_fields(IrType::Int, crate::sb3::VarVal::Int(0), false, flags());
    }
);

crate::instructions_test!(
    mod test_any_local for data_variable {
        fields = super::test::make_fields(IrType::Any, crate::sb3::VarVal::Float(0.0), true, flags());
    }
);

crate::instructions_test!(
    mod test_float_local for data_variable {
        fields = super::test::make_fields(IrType::Float, crate::sb3::VarVal::Float(0.0), true, flags());
    }
);

crate::instructions_test!(
    mod test_string_local for data_variable {
        fields = super::test::make_fields(IrType::String, crate::sb3::VarVal::String("".into()), true, flags());
    }
);

crate::instructions_test!(
    mod test_int_local for data_variable {
        fields = super::test::make_fields(IrType::Int, crate::sb3::VarVal::Int(0), true, flags());
    }
);
