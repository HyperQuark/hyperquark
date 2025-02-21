use crate::ir::RcVar;
use crate::prelude::*;
use wasm_encoder::{ConstExpr, ValType};

use super::{GlobalExportable, GlobalMutable, GlobalRegistry};

pub struct VariableRegistry(Rc<GlobalRegistry>);

impl VariableRegistry {
    fn globals(&self) -> &Rc<GlobalRegistry> {
        &self.0
    }

    pub fn new(globals: &Rc<GlobalRegistry>) -> Self {
        VariableRegistry(Rc::clone(globals))
    }

    pub fn register(&self, var: RcVar) -> HQResult<u32> {
        self.globals().register(
            format!("__rcvar_{:p}", Rc::as_ptr(&var.0)).into(),
            (
                ValType::I64,
                ConstExpr::i64_const(0), // TODO: use the variable's initial value
                GlobalMutable(true),
                GlobalExportable(false),
            ),
        )
    }
}
