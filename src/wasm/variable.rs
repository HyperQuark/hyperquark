use crate::ir::{RcVar, Type as IrType};
use crate::prelude::*;
use wasm_encoder::{ConstExpr, HeapType};

use super::{GlobalExportable, GlobalMutable, GlobalRegistry, WasmProject};

pub struct VariableRegistry(Rc<GlobalRegistry>);

impl VariableRegistry {
    const fn globals(&self) -> &Rc<GlobalRegistry> {
        &self.0
    }

    pub fn new(globals: &Rc<GlobalRegistry>) -> Self {
        Self(Rc::clone(globals))
    }

    pub fn register(&self, var: &RcVar) -> HQResult<u32> {
        self.globals().register(
            format!("__rcvar_{:p}", Rc::as_ptr(&var.0)).into(),
            (
                WasmProject::ir_type_to_wasm(*var.0.possible_types())?,
                match var.0.possible_types().base_type() {
                    Some(IrType::Float) => ConstExpr::f64_const(0.0),
                    Some(IrType::QuasiInt) => ConstExpr::i32_const(0),
                    Some(IrType::String) => ConstExpr::ref_null(HeapType::EXTERN),
                    _ => ConstExpr::i64_const(0), // TODO: use the variable's initial value
                },
                GlobalMutable(true),
                GlobalExportable(false),
            ),
        )
    }
}
