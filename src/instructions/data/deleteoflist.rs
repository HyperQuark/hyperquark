use super::super::prelude::*;
use crate::ir::RcList;

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
    let (list_global, Some(length_global)) = func.registries().lists().register(&fields.list)?
    else {
        hq_bug!("tried to insertatlist of a list with immutable length")
    };
    let array_type = func.registries().lists().array_type(&fields.list)?;
    let index_local = func.local(ValType::I32)?;
    Ok(wasm![
        LocalSet(index_local),
        Block(WasmBlockType::Empty),
        LocalGet(index_local),
        I32Const(0),
        I32LeS,
        BrIf(0),
        LocalGet(index_local),
        #LazyGlobalGet(length_global),
        I32GtS,
        BrIf(0),
        #LazyGlobalGet(list_global),
        LocalGet(index_local),
        I32Const(1),
        I32Sub,
        #LazyGlobalGet(list_global),
        LocalGet(index_local),
        #LazyGlobalGet(length_global),
        LocalGet(index_local),
        I32Sub,
        ArrayCopy {
            array_type_index_dst: array_type,
            array_type_index_src: array_type,
        },
        #LazyGlobalGet(length_global),
        I32Const(1),
        I32Sub,
        #LazyGlobalSet(length_global),
        End,
    ])
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::QuasiInt]))
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test!(
    int_mut;
    data_deleteoflist;
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
    data_deleteoflist;
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
    data_deleteoflist;
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
    data_deleteoflist;
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
