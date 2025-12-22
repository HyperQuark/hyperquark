use super::super::prelude::*;
use crate::ir::RcList;
use crate::wasm::StringsTable;

use wasm_encoder::{BlockType as WasmBlockType, HeapType};

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

#[expect(clippy::too_many_lines, reason = "code generation is just long :(")]
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

    Ok(wasm![Block(WasmBlockType::Result(ValType::Ref(
        RefType::EXTERNREF
    )))]
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
        if elem_type.intersects(
            IrType::StringNan
                .or(IrType::StringNumber),
        ) {
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
            ].into_iter()
            .chain(
                match list.possible_types().base_type() {
                    Some(IrType::String) => {
                        wasm![
                            Call(string_length),
                            I32Const(1),
                            I32Eq,
                        ]
                    },
                    None => {
                        let i64_local = func.local(ValType::I64)?;
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
                    },
                    _ => hq_bug!("shouldn't be checking for single chars in list contents for list with possible types {}", *list.possible_types())
                }
            )
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
            .chain(wasm![
                I32LtS,
                BrIf(1),
                End,
                End,
            ])
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
        Some(IrType::QuasiInt) => {
            let int_to_string = func.registries().external_functions().register(
                ("cast", "int2string".into()),
                (vec![ValType::I32], vec![ValType::EXTERNREF]),
            )?;
            wasm![Call(int_to_string)]
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
        _ => hq_bug!("unexpected list type for list contents, {}", *list.possible_types())
    })
    .chain(wasm![
        Call(string_concat),
        LocalSet(output_local),
        LocalGet(i_local),
        I32Const(1),
        I32Add,
        LocalTee(i_local),
    ]).chain(if let Some(length_global) = maybe_length_global {
        wasm![#LazyGlobalGet(length_global)]
    } else {
        wasm![I32Const(list.initial_value().len().try_into().map_err(
            |_| make_hq_bug!("list initial value length out of bounds")
        )?)]
    }).chain(wasm![
        I32LtS,
        BrIf(0),
        End,
        LocalGet(output_local),
    ])
    .chain(wasm![End])
    .collect())
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    // output type includes string as we return empty string for out-of-bounds
    Ok(Singleton(IrType::String))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test!(
    int_mut;
    data_listcontents;
    @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::QuasiInt,
                vec![],
                &flags()
            );
            *list.length_mutable().borrow_mut() = true;
            list
        },
    };
    { let mut flags = WasmFlags::new(unit_test_wasm_features()); flags.integers = Switch::On; flags }
);
crate::instructions_test!(
    float_mut;
    data_listcontents;
    @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::Float,
                vec![],
                &flags()
            );
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);
crate::instructions_test!(
    string_mut;
    data_listcontents;
    @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::String,
                vec![crate::sb3::VarVal::String("hi".into())],
                &flags()
            );
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);
crate::instructions_test!(
    any_mut;
    data_listcontents;
    @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::Any,
                vec![],
                &flags()
            );
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);

crate::instructions_test!(
    int_static;
    data_listcontents;
    @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::QuasiInt,
            vec![],
            &flags()
        )
    };
    { let mut flags = WasmFlags::new(unit_test_wasm_features()); flags.integers = Switch::On; flags }
);

crate::instructions_test!(
    float_static;
    data_listcontents;
    @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::Float,
            vec![],
            &flags()
        )
    }
);

crate::instructions_test!(
    string_static;
    data_listcontents;
    @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::String,
            vec![],
            &flags()
        )
    }
);

crate::instructions_test!(
    any_static;
    data_listcontents;
    @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::Any,
            vec![],
            &flags()
        )
    }
);
