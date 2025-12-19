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

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let t = inputs[0];
    let (list_global, maybe_length_global) = func.registries().lists().register(&fields.list)?;
    let array_type = func.registries().lists().array_type(&fields.list)?;
    let empty_string = func.registries().strings().register_default("".into())?;
    let local = func.local(WasmProject::ir_type_to_wasm(t)?)?;
    let string_type = IrType::String;
    let elem_type = *fields.list.possible_types();
    let should_box = !IrType::String.contains(elem_type);
    let output_type = WasmProject::ir_type_to_wasm(elem_type.or(string_type))?;
    Ok(if IrType::QuasiInt.contains(t) {
        wasm![
            LocalTee(local),
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
        .chain(wasm![Else, LocalGet(local),])
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
            LocalGet(local),
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
        hq_todo!("non-integer input type for data_itemoflist")
    })
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    // we need to accept strings for last/any/random
    Ok(Rc::from([IrType::Int.or(IrType::String)]))
}

pub fn output_type(inputs: Rc<[IrType]>, Fields { list }: &Fields) -> HQResult<ReturnType> {
    if IrType::String.contains(inputs[0]) {
        hq_todo!("non-integer input type for data_itemoflist")
    }
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
