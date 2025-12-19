use super::super::WasmProject;
use super::{GlobalExportable, GlobalMutable, GlobalRegistry, TypeRegistry};
use crate::ir::{RcList, Type as IrType};
use crate::prelude::*;
use crate::sb3::VarVal;
use crate::wasm::registries::{StringRegistry, TabledStringRegistry};

use wasm_encoder::{ConstExpr, HeapType, Instruction, RefType, StorageType, ValType};

pub struct ListRegistry(
    Rc<GlobalRegistry>,
    Rc<TypeRegistry>,
    Rc<StringRegistry>,
    Rc<TabledStringRegistry>,
);

impl ListRegistry {
    const fn globals(&self) -> &Rc<GlobalRegistry> {
        &self.0
    }

    const fn types(&self) -> &Rc<TypeRegistry> {
        &self.1
    }

    const fn strings(&self) -> &Rc<StringRegistry> {
        &self.2
    }

    const fn tabled_strings(&self) -> &Rc<TabledStringRegistry> {
        &self.3
    }

    #[must_use]
    pub fn new(
        globals: &Rc<GlobalRegistry>,
        types: &Rc<TypeRegistry>,
        strings: &Rc<StringRegistry>,
        tabled_strings: &Rc<TabledStringRegistry>,
    ) -> Self {
        Self(
            Rc::clone(globals),
            Rc::clone(types),
            Rc::clone(strings),
            Rc::clone(tabled_strings),
        )
    }

    pub fn array_type<N>(&self, list: &RcList) -> HQResult<N>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
    {
        let elem_type = WasmProject::ir_type_to_wasm(*list.possible_types())?;
        self.types().array(
            StorageType::Val(elem_type),
            *list.items_mutable().borrow() || *list.length_mutable().borrow(),
        )
    }

    pub fn register<N, M>(&self, list: &RcList) -> HQResult<(N, Option<M>)>
    where
        N: TryFrom<usize>,
        <N as TryFrom<usize>>::Error: fmt::Debug,
        M: TryFrom<usize>,
        <M as TryFrom<usize>>::Error: fmt::Debug,
    {
        let array_type_index = self.array_type(list)?;

        let array_size = if *list.length_mutable().borrow() {
            2000
        } else {
            list.initial_value()
                .len()
                .try_into()
                .map_err(|_| make_hq_bug!("initial list value length out of bounds"))?
        };

        let init_instructions = list
            .initial_value()
            .iter()
            .map(|val| {
                Ok(match list.possible_types().base_type() {
                    Some(IrType::Float) => {
                        let VarVal::Float(f) = val else {
                            hq_bug!("VarVal type should be included in var's possible types")
                        };
                        Instruction::F64Const((*f).into())
                    }
                    Some(IrType::QuasiInt) => {
                        let VarVal::Float(f) = val else {
                            hq_bug!("VarVal type should be included in var's possible types")
                        };
                        hq_assert!(f % 1.0 == 0.0);
                        Instruction::I32Const(*f as i32)
                    }
                    Some(IrType::String) => {
                        let VarVal::String(s) = val else {
                            hq_bug!("VarVal type should be included in var's possible types")
                        };
                        let string_idx = self.strings().register_default(s.clone())?;
                        Instruction::GlobalGet(string_idx)
                    }
                    _ => match val {
                        VarVal::Bool(b) => Instruction::I64Const((*b).into()),
                        VarVal::Float(f) => {
                            Instruction::I64Const(i64::from_le_bytes(f.to_le_bytes()))
                        }
                        VarVal::String(s) => {
                            let string_idx = self.tabled_strings().register_default(s.clone())?;
                            Instruction::I64Const(string_idx)
                        }
                    },
                })
            })
            .collect::<HQResult<Vec<_>>>()?
            .into_iter()
            .chain(core::iter::repeat_n(
                match list.possible_types().base_type() {
                    Some(IrType::QuasiInt) => Instruction::I32Const(0),
                    Some(IrType::Float) => Instruction::F64Const(0.0.into()),
                    Some(IrType::String) => {
                        let string_idx = self.strings().register_default("".into())?;
                        Instruction::GlobalGet(string_idx)
                    }
                    _ => Instruction::I64Const(0),
                },
                2000 - list.initial_value().len(),
            ))
            .chain([Instruction::ArrayNewFixed {
                array_type_index,
                array_size,
            }]);

        let array_global = self.globals().register(
            // format!("__rcvar_{:p}", Rc::as_ptr(&var.0)).into(),
            format!("__rclist_list_{}", list.id()).into(),
            (
                ValType::Ref(RefType {
                    nullable: false,
                    heap_type: HeapType::Concrete(array_type_index),
                }),
                ConstExpr::extended(init_instructions),
                GlobalMutable(*list.items_mutable().borrow() || *list.length_mutable().borrow()),
                GlobalExportable(false),
            ),
        )?;
        let length_global = if *list.length_mutable().borrow() {
            Some(
                self.globals().register(
                    format!("__rclist_len_{}", list.id()).into(),
                    (
                        ValType::I32,
                        ConstExpr::i32_const(
                            array_size
                                .try_into()
                                .map_err(|_| make_hq_bug!("array_size out of bounds"))?,
                        ),
                        GlobalMutable(true),
                        GlobalExportable(false),
                    ),
                )?,
            )
        } else {
            None
        };
        Ok((array_global, length_global))
    }
}
