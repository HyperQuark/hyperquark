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

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
    _fields: &Fields,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

#[cfg(test)]
mod test {
    pub use super::super::listcontents::test_utils::*;
    use super::*;
    use crate::instructions::tests::assert_valid_json;
    use crate::wasm::WasmFlags;

    #[test]
    fn fields_display_is_valid_json() {
        let fields = make_fields(true, flags_with_integers());
        assert_valid_json(format!("{fields}"));
    }

    pub fn make_fields(mutable: bool, flags: WasmFlags) -> Fields {
        Fields {
            list: make_list(mutable, IrType::Any, flags),
        }
    }
}

crate::instructions_test!(
    mod test_mut for data_lengthoflist {
        fields = super::test::make_fields(true, flags());
    }
);

crate::instructions_test!(
    mod test_static for data_lengthoflist {
        fields = super::test::make_fields(false, flags());
    }
);
