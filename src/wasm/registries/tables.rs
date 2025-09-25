use crate::prelude::*;
use wasm_encoder::{
    ConstExpr, ExportKind, ExportSection, HeapType, RefType, TableSection, TableType,
};

#[derive(Clone, Debug)]
pub struct TableOptions {
    pub element_type: RefType,
    pub min: u64,
    pub max: Option<u64>,
    pub init: Option<ConstExpr>,
    pub export_name: Option<&'static str>,
}

pub struct TableRegistrar;
impl RegistryType for TableRegistrar {
    type Key = Box<str>;
    type Value = TableOptions;
}

pub type TableRegistry = NamedRegistry<TableRegistrar>;

impl TableRegistry {
    pub fn finish(self, tables: &mut TableSection, exports: &mut ExportSection) {
        for (
            _key,
            TableOptions {
                element_type,
                min,
                max,
                init: maybe_init,
                export_name,
            },
        ) in self.registry().take()
        {
            // TODO: allow choosing whether to export a table or not?
            if let Some(export_key) = export_name {
                exports.export(export_key, ExportKind::Table, tables.len());
            }
            // let maybe_init = match &*key {
            //     "threads" => Some(ConstExpr::ref_func(imports.len())),
            //     _ => init,
            // };
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

pub struct StringsTable;
impl NamedRegistryItem<TableOptions> for StringsTable {
    const VALUE: TableOptions = TableOptions {
        element_type: RefType::EXTERNREF,
        min: 0,
        // TODO: use js string imports for preknown strings
        max: None,
        init: None,
        export_name: None,
    };
}

pub struct StepsTable;
impl NamedRegistryItem<TableOptions> for StepsTable {
    const VALUE: TableOptions = TableOptions {
        element_type: RefType::FUNCREF,
        min: 0,
        max: None,
        init: None,
        export_name: None,
    };
}
impl NamedRegistryItemOverride<TableOptions, u64> for StepsTable {
    fn r#override(step_count: u64) -> TableOptions {
        TableOptions {
            element_type: RefType::FUNCREF,
            min: step_count,
            max: Some(step_count),
            init: None,
            export_name: None,
        }
    }
}

pub struct ThreadsTable;
impl NamedRegistryItem<TableOptions> for ThreadsTable {
    const VALUE: TableOptions = TableOptions {
        element_type: RefType::FUNCREF,
        min: 0,
        max: None,
        // default to noop, just so the module validates.
        init: None,
        export_name: Some("threads"),
    };
}
impl NamedRegistryItemOverride<TableOptions, (u32, u32, u32)> for ThreadsTable {
    fn r#override(
        (step_func_ty, imported_func_count, static_func_count): (u32, u32, u32),
    ) -> TableOptions {
        TableOptions {
            element_type: RefType {
                nullable: false,
                heap_type: HeapType::Concrete(step_func_ty),
            },
            min: 0,
            max: None,
            init: Some(ConstExpr::ref_func(imported_func_count + static_func_count)),
            export_name: Some("threads"),
        }
    }
}
