use crate::prelude::*;
use crate::registry::MapRegistry;
use core::ops::Deref;
use wasm_encoder::{ConstExpr, ExportKind, ExportSection, GlobalSection, GlobalType, ValType};

#[derive(Copy, Clone, Debug)]
pub struct GlobalMutable(pub bool);

impl Deref for GlobalMutable {
    type Target = bool;
    fn deref(&self) -> &bool {
        &self.0
    }
}

#[derive(Copy, Clone, Debug)]
pub struct GlobalExportable(pub bool);

impl Deref for GlobalExportable {
    type Target = bool;
    fn deref(&self) -> &bool {
        &self.0
    }
}

pub type GlobalRegistry =
    MapRegistry<Box<str>, (ValType, ConstExpr, GlobalMutable, GlobalExportable)>;

impl GlobalRegistry {
    pub fn finish(
        self,
        globals: &mut GlobalSection,
        exports: &mut ExportSection,
        imported_global_count: u32,
        imported_function_count: u32,
        static_function_count: u32,
    ) {
        for (key, (ty, suggested_initial, mutable, export)) in self.registry().take() {
            if *export {
                exports.export(
                    &key,
                    ExportKind::Global,
                    imported_global_count + globals.len(),
                );
            }
            let actual_initial = match &*key {
                "noop_func" => ConstExpr::ref_func(imported_function_count + static_function_count),
                _ => suggested_initial,
            };
            globals.global(
                GlobalType {
                    val_type: ty,
                    mutable: *mutable,
                    shared: false,
                },
                &actual_initial,
            );
        }
    }
}
