use core::ops::Deref;
use core::fmt::Display;

use wasm_encoder::{ConstExpr, ExportKind, ExportSection, GlobalSection, GlobalType, ValType};

use crate::prelude::*;
use crate::registry::MapRegistry;
use crate::wasm::StepTarget;

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
    fn threads_count_with_id<N, S>(&self, id: S) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
        S: Display,
    {
        self.register(
            format!("threads_count{id}").into(),
            (
                ValType::I32,
                ConstExpr::i32_const(0),
                GlobalMutable(true),
                GlobalExportable(true),
            ),
        )
    }

    pub fn threads_count<N>(&self) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.threads_count_with_id("")
    }

    pub fn target_threads_count<N>(&self, target: StepTarget) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.threads_count_with_id(target.suffix_id())
    }

    pub fn finish(
        self,
        globals: &mut GlobalSection,
        exports: &mut ExportSection,
        imported_global_count: u32,
        _imported_function_count: u32,
        _static_function_count: u32,
    ) {
        for (key, (ty, suggested_initial, mutable, export)) in self.registry().take() {
            if *export {
                exports.export(
                    &key,
                    ExportKind::Global,
                    imported_global_count + globals.len(),
                );
            }
            globals.global(
                GlobalType {
                    val_type: ty,
                    mutable: *mutable,
                    shared: false,
                },
                &suggested_initial,
            );
        }
    }
}
