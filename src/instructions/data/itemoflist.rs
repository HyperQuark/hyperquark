use wasm_encoder::BlockType as WasmBlockType;

use super::super::prelude::*;
use crate::ir::RcList;
use crate::wasm::WasmProject;

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
    let output_type = WasmProject::ir_type_to_wasm(elem_type.or(string_type));
    let i32_local = func.local(ValType::I32)?;
    func.free_local(i32_local)?;

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
    Ok(Rc::from([IrType::Int]))
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields { list }: &Fields) -> HQResult<ReturnType> {
    // output type includes string as we return empty string for out-of-bounds
    Ok(Singleton(list.possible_types().or(IrType::String)))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
    _fields: &Fields,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

crate::instructions_test!(
mod int_mut for data_itemoflist(t) {
    fields = super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                vec![],
                &flags()
            ).unwrap();
            *list.length_mutable().borrow_mut() = true;
            list.add_type(IrType::Int);
            list
        },
    };
    flags = { let mut flags = WasmFlags::new(unit_test_wasm_features()); flags.integers = Switch::On; flags };
}
);
crate::instructions_test!(
mod float_mut for data_itemoflist(t) {
    fields = super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                vec![],
                &flags()
            ).unwrap();
            *list.length_mutable().borrow_mut() = true;
            list.add_type(IrType::Float);
            list
        },
    };
}
);
crate::instructions_test!(
mod string_mut for data_itemoflist(t) {
    fields = super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                vec![crate::sb3::VarVal::String("hi".into())],
                &flags()
            ).unwrap();
            *list.length_mutable().borrow_mut() = true;
            list.add_type(IrType::String);
            list
        },
    };
}
);
crate::instructions_test!(
mod any_mut for data_itemoflist(t) {
    fields = super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                vec![],
                &flags()
            ).unwrap();
            *list.length_mutable().borrow_mut() = true;
            list.add_type(IrType::Any);
            list
        },
    };
}
);

crate::instructions_test!(
mod int_static for data_itemoflist(t) {
    fields = super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                vec![],
                &flags()
            ).unwrap();
            list.add_type(IrType::Int);
            list
        }
    };
    flags = { let mut flags = WasmFlags::new(unit_test_wasm_features()); flags.integers = Switch::On; flags };
}
);

crate::instructions_test!(
mod float_static for data_itemoflist(t) {
    fields = super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                vec![],
                &flags()
            ).unwrap();
            list.add_type(IrType::Float);
            list
        }
    };
}
);

crate::instructions_test!(
mod string_static for data_itemoflist(t) {
    fields = super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                vec![],
                &flags()
            ).unwrap();
            list.add_type(IrType::String);
            list
        }
    };
}
);

crate::instructions_test!(
mod any_static for data_itemoflist(t) {
    fields = super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                vec![],
                &flags()
            ).unwrap();
            list.add_type(IrType::Any);
            list
        }
    };
}
);
