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
    inputs: Rc<[IrType]>,
    Fields { list }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let t = inputs[0];
    let (list_global, Some(length_global)) = func.registries().lists().register(list)? else {
        hq_bug!("tried to addtolist of a list with immutable length")
    };
    let local = func.local(WasmProject::ir_type_to_wasm(*list.possible_types())?)?;
    let array_type = func.registries().lists().array_type(list)?;
    Ok(if list.possible_types().is_base_type() {
        vec![]
    } else {
        wasm![@boxed(t)]
    }
    .into_iter()
    .chain(wasm![
        LocalSet(local),
        #LazyGlobalGet(length_global),
        I32Const(200_000),
        I32LtS,
        If(WasmBlockType::Empty),
        #LazyGlobalGet(list_global),
        #LazyGlobalGet(length_global),
        LocalGet(local),
        ArraySet(array_type),
        #LazyGlobalGet(length_global),
        I32Const(1),
        I32Add,
        #LazyGlobalSet(length_global),
        End,
    ])
    .collect())
}

pub fn acceptable_inputs(Fields { list }: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([*list.possible_types()]))
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
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
    int;
    data_addtolist;
    t @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::Int,
                vec![],
                &flags()
            ).unwrap();
            *list.length_mutable().borrow_mut() = true;
            list
        },
    };
    { let mut flags = WasmFlags::new(unit_test_wasm_features()); flags.integers = Switch::On; flags }
);
crate::instructions_test!(
    float;
    data_addtolist;
    t @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::Float,
                vec![],
                &flags()
            ).unwrap();
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);
crate::instructions_test!(
    string;
    data_addtolist;
    t @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::String,
                vec![crate::sb3::VarVal::String("hi".into())],
                &flags()
            ).unwrap();
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);
crate::instructions_test!(
    any;
    data_addtolist;
    t @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::Any,
                vec![],
                &flags()
            ).unwrap();
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);
