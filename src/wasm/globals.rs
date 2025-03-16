use crate::prelude::*;
use crate::registry::MapRegistry;
use core::ops::Deref;
use wasm_encoder::{
    ConstExpr, ExportKind, ExportSection, GlobalSection, GlobalType, ImportSection, ValType,
};

#[derive(Copy, Clone, Debug)]
pub struct Mutable(pub bool);

impl Deref for Mutable {
    type Target = bool;
    fn deref(&self) -> &bool {
        &self.0
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Exportable(pub bool);

impl Deref for Exportable {
    type Target = bool;
    fn deref(&self) -> &bool {
        &self.0
    }
}

pub type GlobalRegistry = MapRegistry<Box<str>, (ValType, ConstExpr, Mutable, Exportable)>;

impl GlobalRegistry {
    pub fn finish(
        self,
        imports: &ImportSection,
        globals: &mut GlobalSection,
        exports: &mut ExportSection,
    ) {
        for (key, (ty, initial, mutable, export)) in self.registry().take() {
            if *export {
                exports.export(&key, ExportKind::Global, globals.len());
            }
            let initial = match &*key {
                "noop_func" => ConstExpr::ref_func(imports.len()),
                _ => initial,
            };
            globals.global(
                GlobalType {
                    val_type: ty,
                    mutable: *mutable,
                    shared: false,
                },
                &initial,
            );
        }
    }
}
