use wasm_encoder::{BlockType as WasmBlockType, HeapType};

use super::super::prelude::*;
use crate::ir::RcList;
use crate::wasm::StringsTable;

/// we need these fields to be mutable for optimisations to be feasible
#[derive(Debug, Clone)]
pub struct Fields {
    pub list: RcList,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "list": {}
    }}"#,
            self.list.borrow(),
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields { list }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let (list_global, maybe_length_global) = func.registries().lists().register(list)?;
    let array_type = func.registries().lists().array_type(list)?;
    let empty_string = func.registries().strings().register_default("".into())?;
    let elem_type = *list.possible_types();
    let is_single_chars_local = func.local(ValType::I32)?;
    let i_local = func.local(ValType::I32)?;
    let output_local = func.local(ValType::Ref(RefType::EXTERNREF))?;
    let space_string = func.registries().strings().register_default(" ".into())?;
    let string_concat = func.registries().external_functions().register(
        ("wasm:js-string", "concat".into()),
        (
            vec![ValType::EXTERNREF, ValType::EXTERNREF],
            vec![ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::EXTERN,
            })],
        ),
    )?;

    let instrs = wasm![
        Block(WasmBlockType::Result(ValType::Ref(RefType::EXTERNREF))),
        I32Const(0),
        LocalSet(i_local),
    ]
    .into_iter()
    .chain(if let Some(length_global) = maybe_length_global {
        wasm![
            #LazyGlobalGet(length_global),
            I32Eqz,
            If(WasmBlockType::Empty),
            GlobalGet(empty_string),
            Br(1),
            End,
        ]
    } else if list.initial_value().is_empty() {
        wasm![GlobalGet(empty_string), Br(0)]
    } else {
        vec![]
    })
    .chain(
        if elem_type.intersects(IrType::StringNan.or(IrType::StringNumber)) {
            let string_length = func.registries().external_functions().register(
                ("wasm:js-string", "length".into()),
                (vec![ValType::EXTERNREF], vec![ValType::I32]),
            )?;
            wasm![
                I32Const(1),
                LocalSet(is_single_chars_local),
                Loop(WasmBlockType::Empty),
                Block(WasmBlockType::Empty),
                #LazyGlobalGet(list_global),
                LocalGet(i_local),
                ArrayGet(array_type),
            ]
            .into_iter()
            .chain(match list.possible_types().base_type() {
                Some(IrType::String) => {
                    wasm![Call(string_length), I32Const(1), I32Eq,]
                }
                None => {
                    let i64_local = func.local(ValType::I64)?;
                    func.free_local(i64_local)?;
                    let strings_table = func.registries().tables().register::<StringsTable, _>()?;
                    wasm![
                        LocalTee(i64_local),
                        I64Const(BOXED_STRING_PATTERN),
                        I64And,
                        I64Const(BOXED_STRING_PATTERN),
                        I64Eq,
                        If(WasmBlockType::Result(ValType::I32)),
                        LocalGet(i64_local),
                        I32WrapI64,
                        TableGet(strings_table),
                        Call(string_length),
                        I32Const(1),
                        I32Eq,
                        Else,
                        I32Const(0),
                        End,
                    ]
                }
                _ => hq_bug!(
                    "shouldn't be checking for single chars in list contents for list with \
                     possible types {}",
                    *list.possible_types()
                ),
            })
            .chain(wasm![
                LocalTee(is_single_chars_local),
                I32Eqz,
                BrIf(0),
                LocalGet(i_local),
                I32Const(1),
                I32Add,
                LocalTee(i_local),
            ])
            .chain(if let Some(length_global) = maybe_length_global {
                wasm![#LazyGlobalGet(length_global)]
            } else {
                wasm![I32Const(list.initial_value().len().try_into().map_err(
                    |_| make_hq_bug!("list initial value length out of bounds")
                )?)]
            })
            .chain(wasm![I32LtS, BrIf(1), End, End,])
            .collect()
        } else {
            wasm![I32Const(0), LocalSet(is_single_chars_local)]
        },
    )
    .chain(wasm![
        I32Const(0),
        LocalSet(i_local),
        GlobalGet(empty_string),
        LocalSet(output_local),
        Loop(WasmBlockType::Empty),
        LocalGet(i_local),
        If(WasmBlockType::Empty),
        LocalGet(is_single_chars_local),
        I32Eqz,
        If(WasmBlockType::Empty),
        LocalGet(output_local),
        GlobalGet(space_string),
        Call(string_concat),
        LocalSet(output_local),
        End,
        End,
        LocalGet(output_local),
        #LazyGlobalGet(list_global),
        LocalGet(i_local),
        ArrayGet(array_type),
    ])
    .chain(match list.possible_types().base_type() {
        Some(IrType::String) => vec![],
        Some(IrType::Float) => {
            let float_to_string = func.registries().external_functions().register(
                ("cast", "float2string".into()),
                (vec![ValType::F64], vec![ValType::EXTERNREF]),
            )?;
            wasm![Call(float_to_string)]
        }
        Some(IrType::Int) => {
            let int_to_string = func.registries().external_functions().register(
                ("cast", "int2string".into()),
                (vec![ValType::I32], vec![ValType::EXTERNREF]),
            )?;
            wasm![Call(int_to_string)]
        }
        Some(IrType::Boolean) => {
            let true_string = func
                .registries()
                .strings()
                .register_default("true".into())?;
            let false_string = func
                .registries()
                .strings()
                .register_default("false".into())?;
            let bool_local = func.local(ValType::I32)?;
            func.free_local(bool_local)?;
            wasm![
                LocalSet(bool_local),
                GlobalGet(true_string),
                GlobalGet(false_string),
                LocalGet(bool_local),
                TypedSelect(ValType::EXTERNREF),
            ]
        }
        None => {
            let float_to_string = func.registries().external_functions().register(
                ("cast", "float2string".into()),
                (vec![ValType::F64], vec![ValType::EXTERNREF]),
            )?;
            let int_to_string = func.registries().external_functions().register(
                ("cast", "int2string".into()),
                (vec![ValType::I32], vec![ValType::EXTERNREF]),
            )?;
            let i64_local = func.local(ValType::I64)?;
            func.free_local(i64_local)?;
            let strings_table = func.registries().tables().register::<StringsTable, _>()?;
            wasm![
                LocalTee(i64_local),
                I64Const(BOXED_STRING_PATTERN),
                I64And,
                I64Const(BOXED_STRING_PATTERN),
                I64Eq,
                If(WasmBlockType::Result(ValType::EXTERNREF)),
                LocalGet(i64_local),
                I32WrapI64,
                TableGet(strings_table),
                Else,
                LocalGet(i64_local),
                I64Const(BOXED_INT_PATTERN),
                I64And,
                I64Const(BOXED_INT_PATTERN),
                I64Eq,
                If(WasmBlockType::Result(ValType::EXTERNREF)),
                LocalGet(i64_local),
                I32WrapI64,
                Call(int_to_string),
                Else,
                LocalGet(i64_local),
                F64ReinterpretI64,
                Call(float_to_string),
                End,
                End,
            ]
        }
        _ => hq_bug!(
            "unexpected list type for list contents, {}",
            *list.possible_types()
        ),
    })
    .chain(wasm![
        Call(string_concat),
        LocalSet(output_local),
        LocalGet(i_local),
        I32Const(1),
        I32Add,
        LocalTee(i_local),
    ])
    .chain(if let Some(length_global) = maybe_length_global {
        wasm![#LazyGlobalGet(length_global)]
    } else {
        wasm![I32Const(list.initial_value().len().try_into().map_err(
            |_| make_hq_bug!("list initial value length out of bounds")
        )?)]
    })
    .chain(wasm![I32LtS, BrIf(0), End, LocalGet(output_local),])
    .chain(wasm![End])
    .collect();

    func.free_local(is_single_chars_local)?;
    func.free_local(i_local)?;
    func.free_local(output_local)?;

    Ok(instrs)
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    Ok(Singleton(IrType::String))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
    _fields: &Fields,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

#[cfg(test)]
pub mod test_utils {
    use super::*;
    use crate::wasm::WasmFlags;
    use crate::wasm::flags::{Switch, unit_test_wasm_features};

    pub fn make_list(mutable: bool, ty: IrType, flags: WasmFlags) -> RcList {
        let list = crate::ir::RcList::new(vec![], &flags).unwrap();
        *list.length_mutable().borrow_mut() = mutable;
        list.add_type(ty);
        list
    }

    pub fn flags_with_integers() -> WasmFlags {
        let mut flags = WasmFlags::new(unit_test_wasm_features());
        flags.integers = Switch::On;
        flags
    }
}

#[cfg(test)]
mod test {
    pub use test_utils::*;

    use super::*;
    use crate::instructions::tests::assert_valid_json;
    use crate::wasm::WasmFlags;

    #[test]
    fn fields_display_is_valid_json() {
        let fields = make_fields(true, IrType::Any, super::test::flags_with_integers());
        assert_valid_json(format!("{fields}"));
    }

    pub fn make_fields(mutable: bool, ty: IrType, flags: WasmFlags) -> Fields {
        Fields {
            list: make_list(mutable, ty, flags),
        }
    }
}

crate::instructions_test!(
    mod test_int_mut for data_listcontents {
        fields = super::test::make_fields(true, IrType::Int, flags());
        flags = super::test::flags_with_integers();
    }
);

crate::instructions_test!(
    mod test_bool_mut for data_listcontents {
        fields = super::test::make_fields(true, IrType::Boolean, flags());
        flags = super::test::flags_with_integers();
    }
);

crate::instructions_test!(
    mod test_float_mut for data_listcontents {
        fields = super::test::make_fields(true, IrType::Float, flags());
    }
);

crate::instructions_test!(
    mod test_string_mut for data_listcontents {
        fields = super::test::make_fields(true, IrType::String, flags());
    }
);

crate::instructions_test!(
    mod test_any_mut for data_listcontents {
        fields = super::test::make_fields(true, IrType::Any, flags());
    }
);

crate::instructions_test!(
    mod test_int_static for data_listcontents {
        fields = super::test::make_fields(false, IrType::Int, flags());
        flags = super::test::flags_with_integers();
    }
);

crate::instructions_test!(
    mod test_bool_static for data_listcontents {
        fields = super::test::make_fields(false, IrType::Boolean, flags());
        flags = super::test::flags_with_integers();
    }
);

crate::instructions_test!(
    mod test_float_static for data_listcontents {
        fields = super::test::make_fields(false, IrType::Float, flags());
    }
);

crate::instructions_test!(
    mod test_string_static for data_listcontents {
        fields = super::test::make_fields(false, IrType::String, flags());
    }
);

crate::instructions_test!(
    mod test_any_static for data_listcontents {
        fields = super::test::make_fields(false, IrType::Any, flags());
    }
);
