use crate::prelude::*;
use crate::registry::MapRegistry;
use wasm_encoder::{ExportKind, ExportSection, RefType, TableSection, TableType};

pub type TableRegistry = MapRegistry<Box<str>, (RefType, u64)>;

impl TableRegistry {
    pub fn finish(self, tables: &mut TableSection, exports: &mut ExportSection) {
        for (key, (element_type, min)) in self.registry().take() {
            // TODO: allow choosing whether to export a table or not?
            exports.export(&key, ExportKind::Table, tables.len());
            // TODO: allow specifying min/max table size when registering, or after registering
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
