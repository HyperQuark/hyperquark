use super::super::prelude::*;
use crate::ir::RcList;

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
    let (_, Some(length_global)) = func.registries().lists().register(list)? else {
        hq_bug!("tried to deletealloflist of a list with immutable length")
    };
    Ok(wasm![I32Const(0), #LazyGlobalSet(length_global)])
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test!(
    test;
    data_deletealloflist;
    @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::Any,
                vec![crate::sb3::VarVal::Float(3.0)],
                &flags()
            );
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);
