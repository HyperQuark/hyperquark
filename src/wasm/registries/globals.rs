use core::ops::Deref;

use wasm_encoder::{
    ConstExpr, ExportKind, ExportSection, GlobalSection, GlobalType, Instruction, ValType,
};

use crate::prelude::*;
use crate::registry::MapRegistry;
use crate::wasm::registries::TypeRegistry;
use crate::wasm::registries::types::{TNonNullable, TTargetThreadArray, TThreadArray, TValType};

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
    pub fn threads_count<N>(&self) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.register(
            "threads_count".into(),
            (
                ValType::I32,
                ConstExpr::i32_const(0),
                GlobalMutable(true),
                GlobalExportable(true),
            ),
        )
    }

    // threadss isn't a typo here - using the Haskell convention of adding extra s's to
    // the end of identifiers for nested lists
    pub fn threadss<N>(&self, types: &Rc<TypeRegistry>, num_sprites: u32) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        let array_array_type = types.register_comp::<TTargetThreadArray, _>()?;
        let array_type = types.register_comp::<TThreadArray, _>()?;
        self.register(
            "threadss".into(),
            (
                <TNonNullable<TTargetThreadArray>>::val_type(&types)?,
                ConstExpr::extended(
                    (0..=num_sprites) // stage + sprites
                        .map(|i| {
                            [
                                Instruction::I32Const(i as i32),
                                Instruction::I32Const(0),
                                Instruction::ArrayNewFixed {
                                    array_type_index: array_type,
                                    array_size: 0,
                                },
                            ]
                        })
                        .flatten()
                        .chain([Instruction::ArrayNewFixed {
                            array_type_index: array_array_type,
                            array_size: num_sprites,
                        }]),
                ), // TODO: initialise properly
                GlobalMutable(true),
                GlobalExportable(false),
            ),
        )
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
