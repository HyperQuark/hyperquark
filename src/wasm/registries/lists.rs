use super::super::WasmProject;
use super::{GlobalExportable, GlobalMutable, GlobalRegistry, TypeRegistry};
use crate::ir::{RcList, Type as IrType};
use crate::prelude::*;
use crate::registry::MapRegistry;
use crate::sb3::VarVal;
use crate::wasm::registries::{StringRegistry, TabledStringRegistry};

use wasm_encoder::{
    ConstExpr, DataSection, ElementSection, Elements, Function, HeapType, Instruction, RefType,
    StorageType, ValType,
};

#[derive(Clone)]
pub struct ListRegistry(
    MapRegistry<RcList, u32>, // for keeping track of initialisers
    Rc<GlobalRegistry>,
    Rc<TypeRegistry>,
    Rc<StringRegistry>,
    Rc<TabledStringRegistry>,
);

impl ListRegistry {
    const fn registry(&self) -> &MapRegistry<RcList, u32> {
        &self.0
    }

    const fn globals(&self) -> &Rc<GlobalRegistry> {
        &self.1
    }

    const fn types(&self) -> &Rc<TypeRegistry> {
        &self.2
    }

    const fn strings(&self) -> &Rc<StringRegistry> {
        &self.3
    }

    const fn tabled_strings(&self) -> &Rc<TabledStringRegistry> {
        &self.4
    }

    #[must_use]
    pub fn new(
        globals: &Rc<GlobalRegistry>,
        types: &Rc<TypeRegistry>,
        strings: &Rc<StringRegistry>,
        tabled_strings: &Rc<TabledStringRegistry>,
    ) -> Self {
        Self(
            MapRegistry::default(),
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
        self.types().array(StorageType::Val(elem_type), true)
    }

    pub fn register<M>(&self, list: &RcList) -> HQResult<(u32, Option<M>)>
    where
        M: TryFrom<usize>,
        <M as TryFrom<usize>>::Error: fmt::Debug,
    {
        let array_type_index = self.array_type(list)?;

        let initial_length = list
            .initial_value()
            .len()
            .try_into()
            .map_err(|_| make_hq_bug!("initial list value length out of bounds"))?;

        let array_size = if *list.length_mutable().borrow() {
            200_000
        } else {
            initial_length
        };

        let init_instructions = [
            match list.possible_types().base_type() {
                Some(IrType::QuasiInt) => Instruction::I32Const(0),
                Some(IrType::Float) => Instruction::F64Const(0.0.into()),
                Some(IrType::String) => {
                    let string_idx = self.strings().register_default("".into())?;
                    Instruction::GlobalGet(string_idx)
                }
                _ => Instruction::I64Const(0),
            },
            Instruction::I32Const(array_size),
            Instruction::ArrayNew(array_type_index),
        ];

        let array_global = self.globals().register(
            // format!("__rcvar_{:p}", Rc::as_ptr(&var.0)).into(),
            format!("__rclist_list_{}", list.id()).into(),
            (
                ValType::Ref(RefType {
                    nullable: false,
                    heap_type: HeapType::Concrete(array_type_index),
                }),
                ConstExpr::extended(init_instructions),
                GlobalMutable(*list.length_mutable().borrow()),
                GlobalExportable(false),
            ),
        )?;

        self.registry()
            .register::<usize>(list.clone(), array_global)?;

        // make sure strings are registered before we call `finish`, as `finish` needs to be
        // called after the strings registry is finished.
        match list.possible_types().base_type() {
            Some(IrType::String) => {
                list.initial_value().iter().try_for_each(|val| {
                    let VarVal::String(s) = val else {
                        hq_bug!("VarVal type should be included in var's possible types")
                    };
                    self.strings().register_default::<usize>(s.clone())?;
                    Ok(())
                })?;
            }
            None => {
                list.initial_value()
                    .iter()
                    .try_for_each::<_, HQResult<()>>(|val| {
                        if let VarVal::String(s) = val {
                            self.tabled_strings().register_default::<usize>(s.clone())?;
                        }
                        Ok(())
                    })?;
            }
            _ => (),
        }

        let length_global = if *list.length_mutable().borrow() {
            Some(self.globals().register(
                format!("__rclist_len_{}", list.id()).into(),
                (
                    ValType::I32,
                    ConstExpr::i32_const(initial_length),
                    GlobalMutable(true),
                    GlobalExportable(false),
                ),
            )?)
        } else {
            None
        };

        Ok((array_global, length_global))
    }

    pub fn finish(
        self,
        data_section: &mut DataSection,
        elem_section: &mut ElementSection,
        start_func: &mut Function,
        imported_global_count: u32,
    ) -> HQResult<()> {
        for (list, &array_global) in self.registry().registry().borrow().iter() {
            start_func
                .instruction(&Instruction::GlobalGet(
                    array_global + imported_global_count,
                ))
                .instruction(&Instruction::I32Const(0))
                .instruction(&Instruction::I32Const(0))
                .instruction(&Instruction::I32Const(
                    list.initial_value()
                        .len()
                        .try_into()
                        .map_err(|_| make_hq_bug!("list initial value length out of bounds"))?,
                ));

            let array_type_index = self.array_type(list)?;

            match list.possible_types().base_type() {
                Some(IrType::Float) => {
                    let floats_bytes = list
                        .initial_value()
                        .iter()
                        .map(|val| {
                            let VarVal::Float(f) = val else {
                                hq_bug!("VarVal type should be included in var's possible types")
                            };
                            Ok(f.to_le_bytes())
                        })
                        .collect::<HQResult<Box<[_]>>>()?
                        .into_iter()
                        .flatten()
                        .collect::<Box<[_]>>();

                    data_section.passive(floats_bytes);

                    start_func.instruction(&Instruction::ArrayInitData {
                        array_type_index,
                        array_data_index: data_section.len() - 1,
                    });
                }
                Some(IrType::QuasiInt) => {
                    let ints_bytes = list
                        .initial_value()
                        .iter()
                        .map(|val| {
                            Ok(match val {
                                #[expect(
                                    clippy::cast_possible_truncation,
                                    reason = "integer-ness already confirmed; `as` is saturating."
                                )]
                                VarVal::Float(f) => {
                                    hq_assert!(f % 1.0 == 0.0);
                                    *f as i32
                                }
                                VarVal::Int(i) => *i,
                                VarVal::Bool(b) => (*b).into(),
                                VarVal::String(_) => {
                                    hq_bug!(
                                        "VarVal type should be included in var's possible types"
                                    )
                                }
                            }
                            .to_le_bytes())
                        })
                        .collect::<HQResult<Box<[_]>>>()?
                        .into_iter()
                        .flatten()
                        .collect::<Box<[_]>>();

                    data_section.passive(ints_bytes);

                    start_func.instruction(&Instruction::ArrayInitData {
                        array_type_index,
                        array_data_index: data_section.len() - 1,
                    });
                }
                Some(IrType::String) => {
                    let strings = list
                        .initial_value()
                        .iter()
                        .map(|val| {
                            let VarVal::String(s) = val else {
                                hq_bug!("VarVal type should be included in var's possible types")
                            };
                            let string_idx = self.strings().register_default(s.clone())?;
                            Ok(ConstExpr::global_get(string_idx))
                        })
                        .collect::<HQResult<Box<[_]>>>()?;

                    elem_section.passive(Elements::Expressions(
                        RefType::EXTERNREF,
                        Cow::Borrowed(&strings),
                    ));

                    start_func.instruction(&Instruction::ArrayInitElem {
                        array_type_index,
                        array_elem_index: elem_section.len() - 1,
                    });
                }
                _ => {
                    let boxed_bytes = list
                        .initial_value()
                        .iter()
                        .map(|val| {
                            Ok(match val {
                                VarVal::Int(i) => (*i).into(),
                                VarVal::Bool(b) => (*b).into(),
                                VarVal::Float(f) => i64::from_le_bytes(f.to_le_bytes()),
                                VarVal::String(s) => {
                                    self.tabled_strings().register_default(s.clone())?
                                }
                            }
                            .to_le_bytes())
                        })
                        .collect::<HQResult<Box<[_]>>>()?
                        .into_iter()
                        .flatten()
                        .collect::<Box<[_]>>();

                    data_section.passive(boxed_bytes);

                    start_func.instruction(&Instruction::ArrayInitData {
                        array_type_index,
                        array_data_index: data_section.len() - 1,
                    });
                }
            }
        }

        Ok(())
    }
}
