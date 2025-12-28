use crate::registry::SetRegistry;
use crate::wasm::WasmProject;
use crate::{ir::RcVar, prelude::*};
use wasm_encoder::{
    AbstractHeapType, FieldType, HeapType, RefType, StorageType, TypeSection, ValType,
};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum WasmType {
    Function(Vec<ValType>, Vec<ValType>),
    Array(StorageType, bool),
    Struct(Vec<FieldType>),
}

pub type TypeRegistry = SetRegistry<WasmType>;

impl TypeRegistry {
    pub fn function<N>(&self, params: Vec<ValType>, returns: Vec<ValType>) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.register_default(WasmType::Function(params, returns))
    }

    pub fn array<N>(&self, elem_type: StorageType, mutable: bool) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.register_default(WasmType::Array(elem_type, mutable))
    }

    pub fn struct_<N>(&self, fields: Vec<FieldType>) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        self.register_default(WasmType::Struct(fields))
    }

    pub const STRUCT_REF: ValType = ValType::Ref(RefType {
        nullable: true,
        heap_type: HeapType::Abstract {
            shared: false,
            ty: AbstractHeapType::Struct,
        },
    });

    pub fn step_func_type(&self) -> HQResult<u32> {
        self.function(vec![ValType::I32, Self::STRUCT_REF], vec![])
    }

    pub fn stack_struct_type(&self) -> HQResult<u32> {
        self.struct_(vec![
            FieldType {
                element_type: StorageType::Val(ValType::Ref(RefType {
                    nullable: false,
                    heap_type: HeapType::Concrete(self.step_func_type()?),
                })),
                mutable: true,
            },
            FieldType {
                element_type: StorageType::Val(Self::STRUCT_REF),
                mutable: false,
            },
        ])
    }

    pub fn stack_array_type(&self) -> HQResult<u32> {
        self.array(
            StorageType::Val(ValType::Ref(RefType {
                nullable: true,
                heap_type: HeapType::Concrete(self.stack_struct_type()?),
            })),
            true,
        )
    }

    pub fn thread_struct_type(&self) -> HQResult<u32> {
        self.struct_(vec![
            FieldType {
                element_type: StorageType::Val(ValType::I32),
                mutable: true,
            },
            FieldType {
                element_type: StorageType::Val(ValType::Ref(RefType {
                    nullable: false,
                    heap_type: HeapType::Concrete(self.stack_array_type()?),
                })),
                mutable: true,
            },
        ])
    }

    pub fn proc_arg_struct_type(
        &self,
        arg_vars: &core::cell::Ref<'_, Vec<RcVar>>,
    ) -> HQResult<u32> {
        self.struct_(
            arg_vars
                .iter()
                .map(|var| {
                    Ok(FieldType {
                        mutable: false,
                        element_type: StorageType::Val(WasmProject::ir_type_to_wasm(
                            *var.possible_types(),
                        )?),
                    })
                })
                .collect::<HQResult<Vec<_>>>()?,
        )
    }

    pub fn finish(self, types: &mut TypeSection) {
        for ty in self.registry().take().keys().cloned() {
            match ty {
                WasmType::Function(params, results) => types.ty().function(params, results),
                WasmType::Array(elem_type, mutable) => types.ty().array(&elem_type, mutable),
                WasmType::Struct(fields) => types.ty().struct_(fields),
            }
        }
    }
}
