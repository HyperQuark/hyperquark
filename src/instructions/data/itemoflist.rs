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
    _inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let (list_global, maybe_length_global) = func.registries().lists().register(&fields.list)?;
    let array_type = func.registries().lists().array_type(&fields.list)?;
    let empty_string = func.registries().strings().register_default("".into())?;
    let string_type = IrType::String;
    let elem_type = *fields.list.possible_types();
    let should_box = !IrType::String.contains(elem_type);
    let output_type = WasmProject::ir_type_to_wasm(elem_type.or(string_type))?;
    let i32_local = func.local(ValType::I32)?;

    Ok(wasm![
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
    .collect())
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::QuasiInt]))
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
    data_itemoflist;
    t @ super::Fields {
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
    data_itemoflist;
    t @ super::Fields {
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
    data_itemoflist;
    t @ super::Fields {
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
    data_itemoflist;
    t @ super::Fields {
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
    data_itemoflist;
    t @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::Float,
            vec![],
            &flags()
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
            &flags()
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
            &flags()
        )
    }
);
