use crate::prelude::*;
use crate::registry::SetRegistry;
use wasm_encoder::{EntityType, Function, GlobalType, ImportSection, Instruction, ValType};

#[derive(Clone, Default)]
pub struct StringRegistry(SetRegistry<Box<str>>);

impl StringRegistry {
    pub fn register_default<N>(&self, string: Box<str>) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        // crate::log!("registering string: {}", string);
        self.0.register_default(string)
    }

    pub fn registry(&self) -> &RefCell<IndexMap<Box<str>, ()>> {
        self.0.registry()
    }

    pub fn finish(self, imports: &mut ImportSection) {
        for string in self.0.registry().take().keys() {
            imports.import(
                "",
                string,
                EntityType::Global(GlobalType {
                    val_type: ValType::EXTERNREF,
                    mutable: false,
                    shared: false,
                }),
            );
        }
    }
}

#[derive(Clone, Default)]
pub struct TabledStringRegistry(SetRegistry<Box<str>>);

impl TabledStringRegistry {
    pub fn register_default<N>(&self, string: Box<str>) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.0.register_default(string)
    }

    pub fn registry(&self) -> &RefCell<IndexMap<Box<str>, ()>> {
        self.0.registry()
    }

    pub fn finish(
        self,
        strings: &StringRegistry,
        strings_table: u32,
        start_func: &mut Function,
    ) -> HQResult<()> {
        for string in self.registry().take().keys() {
            let string_idx = strings.register_default(string.clone())?;

            start_func
                .instruction(&Instruction::GlobalGet(string_idx))
                .instruction(&Instruction::I32Const(1))
                .instruction(&Instruction::TableGrow(strings_table))
                .instruction(&Instruction::Drop);
        }
        Ok(())
    }
}
