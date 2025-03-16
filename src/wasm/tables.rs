use crate::prelude::*;
use crate::registry::MapRegistry;
use wasm_encoder::{
    ConstExpr, ExportKind, ExportSection, ImportSection, RefType, TableSection, TableType,
};

pub type TableRegistry = MapRegistry<Box<str>, (RefType, u64, Option<ConstExpr>)>;

impl TableRegistry {
    pub fn finish(
        self,
        imports: &ImportSection,
        tables: &mut TableSection,
        exports: &mut ExportSection,
    ) {
        for (key, (element_type, min, init)) in self.registry().take() {
            // TODO: allow choosing whether to export a table or not?
            exports.export(&key, ExportKind::Table, tables.len());
            let init = match &*key {
                "threads" => Some(ConstExpr::ref_func(imports.len())),
                _ => init,
            };
            if let Some(init) = init {
                tables.table_with_init(
                    TableType {
                        element_type,
                        minimum: min,
                        maximum: None,
                        table64: false,
                        shared: false,
                    },
                    &init,
                );
            } else {
                tables.table(TableType {
                    element_type,
                    minimum: min,
                    maximum: None,
                    table64: false,
                    shared: false,
                });
            }
        }
    }
}
