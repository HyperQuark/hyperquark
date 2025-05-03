use crate::prelude::*;
use crate::registry::MapRegistry;
use wasm_encoder::{
    ConstExpr, ExportKind, ExportSection, ImportSection, RefType, TableSection, TableType,
};

#[derive(Clone, Debug)]
pub struct TableOptions {
    pub element_type: RefType,
    pub min: u64,
    pub max: Option<u64>,
    pub init: Option<ConstExpr>,
}

pub type TableRegistry = MapRegistry<Box<str>, TableOptions>;

impl TableRegistry {
    pub fn finish(
        self,
        imports: &ImportSection,
        tables: &mut TableSection,
        exports: &mut ExportSection,
    ) {
        for (
            key,
            TableOptions {
                element_type,
                min,
                max,
                init,
            },
        ) in self.registry().take()
        {
            // TODO: allow choosing whether to export a table or not?
            exports.export(&key, ExportKind::Table, tables.len());
            let maybe_init = match &*key {
                "threads" => Some(ConstExpr::ref_func(imports.len())),
                _ => init,
            };
            if let Some(init) = maybe_init {
                tables.table_with_init(
                    TableType {
                        element_type,
                        minimum: min,
                        maximum: max,
                        table64: false,
                        shared: false,
                    },
                    &init,
                );
            } else {
                tables.table(TableType {
                    element_type,
                    minimum: min,
                    maximum: max,
                    table64: false,
                    shared: false,
                });
            }
        }
    }
}
