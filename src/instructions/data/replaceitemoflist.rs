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
    hq_assert!(inputs.len() == 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    let (list_global, maybe_length_global) = func.registries().lists().register(&fields.list)?;
    let array_type = func.registries().lists().array_type(&fields.list)?;
    let index_local = func.local(WasmProject::ir_type_to_wasm(t1)?)?;
    let val_local = func.local(WasmProject::ir_type_to_wasm(t2)?)?;
    Ok(if IrType::QuasiInt.contains(t1) {
        wasm![
            LocalSet(val_local),
            LocalSet(index_local),
            Block(WasmBlockType::Empty),
            LocalGet(index_local),
            I32Const(0),
            I32LeS,
            BrIf(0),
            LocalGet(index_local),
        ]
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
            I32GtS,
            BrIf(0),
            #LazyGlobalGet(list_global),
            LocalGet(index_local),
            LocalGet(val_local),
        ])
        .chain(if fields.list.possible_types().is_base_type() {
            vec![]
        } else {
            wasm![@boxed(t2)]
        })
        .chain(wasm![ArraySet(array_type), End,])
        .collect()
    } else {
        hq_todo!("non-integer input type for data_replaceitemoflist")
    })
}

pub fn acceptable_inputs(Fields { list }: &Fields) -> HQResult<Rc<[IrType]>> {
    // we need to accept strings for last/any/random
    Ok(Rc::from([
        IrType::Int.or(IrType::String),
        *list.possible_types(),
    ]))
}

pub fn output_type(inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    if IrType::String.contains(inputs[0]) {
        hq_todo!("non-integer input type for 1st input of data_replaceitemoflist")
    }
    // output type includes string as we return empty string for out-of-bounds
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test!(
    int_mut;
    data_replaceitemoflist;
    t1, t2 @ super::Fields {
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
    data_replaceitemoflist;
    t1, t2 @ super::Fields {
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
    data_replaceitemoflist;
    t1, t2 @ super::Fields {
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
    data_replaceitemoflist;
    t1, t2 @ super::Fields {
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
    data_replaceitemoflist;
    t1, t2 @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::QuasiInt,
            vec![],
        )
    }
);

crate::instructions_test!(
    float_static;
    data_replaceitemoflist;
    t1, t2 @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::Float,
            vec![],
        )
    }
);

crate::instructions_test!(
    string_static;
    data_replaceitemoflist;
    t1, t2 @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::String,
            vec![],
        )
    }
);

crate::instructions_test!(
    any_static;
    data_replaceitemoflist;
    t1, t2 @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::Any,
            vec![],
        )
    }
);
