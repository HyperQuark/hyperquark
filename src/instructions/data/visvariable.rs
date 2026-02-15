/// this is currently just a convenience block. if we get thread-scoped variables
/// (i.e. locals) then this can actually use the wasm tee instruction.
use super::super::prelude::*;
use crate::ir::RcVar;

#[derive(Debug, Clone)]
pub struct Fields {
    pub var: RefCell<RcVar>,
    pub visible: bool,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "variable": {},
        "visible": {}
    }}"#,
            self.var.borrow(),
            self.visible,
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields { var, visible }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let borrowed_var = var.borrow();
    let Some(monitor) = borrowed_var.monitor().as_ref() else {
        hq_bug!("tried to change visibility of variable without monitor")
    };
    hq_assert!(
        *monitor.is_ever_visible.borrow(),
        "tried to change visibility of unused monitor"
    );
    let update_func = func.registries().external_functions().register(
        ("data", "update_var_visible".into()),
        (vec![ValType::EXTERNREF, ValType::I32], vec![]),
    )?;
    let variable_string = func
        .registries()
        .strings()
        .register_default(monitor.id.clone())?;
    Ok(wasm![
        GlobalGet(variable_string),
        I32Const((*visible).into()),
        Call(update_func),
    ])
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
    _fields: &Fields,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

crate::instructions_test!(
    test;
    data_visvariable;
    @ super::Fields {
        var: RefCell::new(
                crate::ir::RcVar::new(
                    IrType::Any,
                    crate::sb3::VarVal::Float(0.0),
                    Some(crate::ir::IrMonitor {
                        id: "".into(),
                        is_ever_visible: RefCell::new(true,)
                    }),
                ).unwrap())
            ,
        visible: true
    }
);
