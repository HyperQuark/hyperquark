use super::super::WasmProject;
use super::{GlobalExportable, GlobalMutable, GlobalRegistry};
use crate::ir::{RcVar, Type as IrType};
use crate::prelude::*;
use crate::sb3::VarVal;
use crate::wasm::registries::{StringRegistry, TabledStringRegistry};
use wasm_encoder::ConstExpr;

pub struct VariableRegistry(
    Rc<GlobalRegistry>,
    Rc<StringRegistry>,
    Rc<TabledStringRegistry>,
);

impl VariableRegistry {
    const fn globals(&self) -> &Rc<GlobalRegistry> {
        &self.0
    }

    const fn strings(&self) -> &Rc<StringRegistry> {
        &self.1
    }

    const fn tabled_strings(&self) -> &Rc<TabledStringRegistry> {
        &self.2
    }

    #[must_use]
    pub fn new(
        globals: &Rc<GlobalRegistry>,
        strings: &Rc<StringRegistry>,
        tabled_strings: &Rc<TabledStringRegistry>,
    ) -> Self {
        Self(
            Rc::clone(globals),
            Rc::clone(strings),
            Rc::clone(tabled_strings),
        )
    }

    pub fn register<N>(&self, var: &RcVar) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.globals().register(
            // format!("__rcvar_{:p}", Rc::as_ptr(&var.0)).into(),
            format!("__rcvar_{}", var.id()).into(),
            (
                WasmProject::ir_type_to_wasm(*var.possible_types())?,
                match var.possible_types().base_type() {
                    Some(IrType::Float) => {
                        let VarVal::Float(f) = var.initial_value() else {
                            hq_bug!("VarVal type should be included in var's possible types")
                        };
                        ConstExpr::f64_const((*f).into())
                    }
                    Some(IrType::QuasiInt) => match var.initial_value() {
                        VarVal::Float(f) => {
                            hq_assert!(f % 1.0 == 0.0);
                            ConstExpr::i32_const(*f as i32)
                        }
                        VarVal::Int(i) => ConstExpr::i32_const(*i),
                        VarVal::Bool(b) => ConstExpr::i32_const((*b).into()),
                        VarVal::String(_) => {
                            hq_bug!("VarVal type should be included in var's possible types")
                        }
                    },
                    Some(IrType::String) => {
                        let VarVal::String(s) = var.initial_value() else {
                            hq_bug!("VarVal type should be included in var's possible types")
                        };
                        let string_idx = self.strings().register_default(s.clone())?;
                        ConstExpr::global_get(string_idx)
                    }
                    _ => match var.initial_value() {
                        VarVal::Int(i) => ConstExpr::i64_const((*i).into()),
                        VarVal::Bool(b) => ConstExpr::i64_const((*b).into()),
                        VarVal::Float(f) => {
                            ConstExpr::i64_const(i64::from_le_bytes(f.to_le_bytes()))
                        }
                        VarVal::String(s) => {
                            let string_idx = self.tabled_strings().register_default(s.clone())?;
                            ConstExpr::i64_const(string_idx)
                        }
                    },
                },
                GlobalMutable(true),
                GlobalExportable(false),
            ),
        )
    }
}
