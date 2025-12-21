use super::super::prelude::*;
use crate::ir::RcList;
use crate::wasm::WasmProject;

use wasm_encoder::BlockType as WasmBlockType;

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
    inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let t = inputs[0];
    let (list_global, maybe_length_global) = func.registries().lists().register(&fields.list)?;
    let array_type = func.registries().lists().array_type(&fields.list)?;
    let empty_string = func.registries().strings().register_default("".into())?;
    let string_type = IrType::String;
    let elem_type = *fields.list.possible_types();
    let should_box = !IrType::String.contains(elem_type);
    let output_type = WasmProject::ir_type_to_wasm(elem_type.or(string_type))?;
    // although we specify QuasiInt or String in acceptable_inputs, we get some floats slipping through.
    // todo: why???
    Ok(if IrType::Number.contains(t) {
        let i32_local = func.local(ValType::I32)?;
        wasm![
            LocalTee(i32_local),
            I32Const(0),
            I32LeS,
            If(WasmBlockType::Result(output_type)),
            GlobalGet(empty_string),
        ]
        .into_iter()
        .chain(if should_box {
            wasm![
                @boxed(string_type) ]
        } else {
            vec![]
        })
        .chain(wasm![Else, LocalGet(i32_local),])
        .chain(if let Some(length_global) = maybe_length_global {
            wasm![#LazyGlobalGet(length_global)]
        } else {
            let array_length = fields
                .list
                .initial_value()
                .len()
                .try_into()
                .map_err(|_| make_hq_bug!("list initial value length out of bounds"))?;
            wasm![I32Const(array_length)]
        })
        .chain(wasm![
            I32GtS,
            If(WasmBlockType::Result(output_type)),
            GlobalGet(empty_string),
        ])
        .chain(if should_box {
            wasm![
                @boxed(string_type) ]
        } else {
            vec![]
        })
        .chain(wasm![
            Else,
            #LazyGlobalGet(list_global),
            LocalGet(i32_local),
            I32Const(1),
            I32Sub,
            ArrayGet(array_type),
        ])
        .chain(if should_box {
            wasm![
                @boxed(elem_type) ]
        } else {
            vec![]
        })
        .chain(wasm![End, End,])
        .collect()
    } else {
        let cast_func_index = func.registries().external_functions().register(
            ("cast", "string2float".into()),
            (vec![ValType::EXTERNREF], vec![ValType::F64]),
        )?;
        let float_type = IrType::Float;
        let string_local = func.local(ValType::EXTERNREF)?;
        let f64_local = func.local(ValType::F64)?;
        let i32_local = func.local(ValType::I32)?;
        let length_local = func.local(ValType::I32)?;
        let string_eq = func.registries().external_functions().register(
            ("operator", "eq_string".into()),
            (
                vec![ValType::EXTERNREF, ValType::EXTERNREF],
                vec![ValType::I32],
            ),
        )?;
        let last_string = func
            .registries()
            .strings()
            .register_default("last".into())?;
        let any_string = func.registries().strings().register_default("any".into())?;
        let random_string = func
            .registries()
            .strings()
            .register_default("random".into())?;
        let random = func
            .registries()
            .external_functions()
            .register(("operator", "random".into()), (vec![], vec![ValType::F64]))?;
        wasm![LocalSet(string_local),]
            .into_iter()
            .chain(if let Some(length_global) = maybe_length_global {
                wasm![#LazyGlobalGet(length_global)]
            } else {
                let array_length = fields
                    .list
                    .initial_value()
                    .len()
                    .try_into()
                    .map_err(|_| make_hq_bug!("list initial value length out of bounds"))?;
                wasm![I32Const(array_length)]
            })
            .chain(wasm![
                LocalSet(length_local),
                LocalGet(string_local),
                Call(cast_func_index),
                LocalTee(f64_local),
                @isnan(float_type),
                If(WasmBlockType::Result(output_type)),
                    LocalGet(string_local),
                    GlobalGet(last_string),
                    Call(string_eq),
                    If(WasmBlockType::Result(output_type)),
                        #LazyGlobalGet(list_global),
                        LocalGet(length_local),
                        I32Const(1),
                        I32Sub,
                        ArrayGet(array_type),
            ])
            .chain(if should_box {
                wasm![
                @boxed(elem_type) ]
            } else {
                vec![]
            })
            .chain(wasm![
                Else,
                LocalGet(string_local),
                GlobalGet(any_string),
                Call(string_eq),
                LocalGet(string_local),
                GlobalGet(random_string),
                Call(string_eq),
                I32Or,
                If(WasmBlockType::Result(output_type)),
                #LazyGlobalGet(list_global),
                LocalGet(length_local),
                F64ConvertI32S,
                Call(random),
                F64Mul,
                I32TruncSatF64S,
                ArrayGet(array_type),
            ])
            .chain(if should_box {
                wasm![
                @boxed(elem_type) ]
            } else {
                vec![]
            })
            .chain(wasm![Else, GlobalGet(empty_string),])
            .chain(if should_box {
                wasm![
                @boxed(string_type) ]
            } else {
                vec![]
            })
            .chain(wasm![
                End,
                End,
                Else,
                LocalGet(f64_local),
                I32TruncSatF64S,
                LocalTee(i32_local),
                I32Const(0),
                I32LeS,
                If(WasmBlockType::Result(output_type)),
                GlobalGet(empty_string),
            ])
            .chain(if should_box {
                wasm![
                @boxed(string_type) ]
            } else {
                vec![]
            })
            .chain(wasm![
                Else,
                LocalGet(i32_local),
                LocalGet(length_local),
                I32GtS,
                If(WasmBlockType::Result(output_type)),
                GlobalGet(empty_string),
            ])
            .chain(if should_box {
                wasm![
                @boxed(string_type) ]
            } else {
                vec![]
            })
            .chain(wasm![
                    Else,
                        #LazyGlobalGet(list_global),
                        LocalGet(i32_local),
                        I32Const(1),
                        I32Sub,
                        ArrayGet(array_type),
            ])
            .chain(if should_box {
                wasm![
                @boxed(elem_type) ]
            } else {
                vec![]
            })
            .chain(wasm![End, End, End])
            .collect()
    })
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    // we need to accept strings for last/any/random
    Ok(Rc::from([IrType::QuasiInt.or(IrType::String)]))
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields { list }: &Fields) -> HQResult<ReturnType> {
    // output type includes string as we return empty string for out-of-bounds
    Ok(Singleton(list.possible_types().or(IrType::String)))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test!(
    int_mut;
    data_itemoflist;
    t @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::QuasiInt,
                vec![],
            );
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);
crate::instructions_test!(
    float_mut;
    data_itemoflist;
    t @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::Float,
                vec![],
            );
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);
crate::instructions_test!(
    string_mut;
    data_itemoflist;
    t @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::String,
                vec![crate::sb3::VarVal::String("hi".into())],
            );
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);
crate::instructions_test!(
    any_mut;
    data_itemoflist;
    t @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::Any,
                vec![],
            );
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);

crate::instructions_test!(
    int_static;
    data_itemoflist;
    t @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::QuasiInt,
            vec![],
        )
    }
);

crate::instructions_test!(
    float_static;
    data_itemoflist;
    t @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::Float,
            vec![],
        )
    }
);

crate::instructions_test!(
    string_static;
    data_itemoflist;
    t @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::String,
            vec![],
        )
    }
);

crate::instructions_test!(
    any_static;
    data_itemoflist;
    t @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::Any,
            vec![],
        )
    }
);
