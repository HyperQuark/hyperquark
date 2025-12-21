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
    Ok(
        if let (_, Some(length_global)) = func.registries().lists().register(list)? {
            wasm![#LazyGlobalGet(length_global)]
        } else {
            let array_length = list
                .initial_value()
                .len()
                .try_into()
                .map_err(|_| make_hq_bug!("list initial value length out of bounds"))?;
            wasm![I32Const(array_length)]
        },
    )
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields { list }: &Fields) -> HQResult<ReturnType> {
    Ok(Singleton(
        if !list.initial_value().is_empty() && !*list.length_mutable().borrow() {
            IrType::IntPos
        } else {
            IrType::IntPos.or(IrType::IntZero)
        },
    ))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test!(
    _mut;
    data_lengthoflist;
    @ super::Fields {
        list: {
            let list = crate::ir::RcList::new(
                IrType::Any,
                vec![crate::sb3::VarVal::Float(3.0)],
            );
            *list.length_mutable().borrow_mut() = true;
            list
        },
    }
);

crate::instructions_test!(
    _static;
    data_lengthoflist;
    @ super::Fields {
        list: crate::ir::RcList::new(
            IrType::Any,
            vec![crate::sb3::VarVal::Float(3.0)],
        ),
    }
);
