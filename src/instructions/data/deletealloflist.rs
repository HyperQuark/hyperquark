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
        let fields = make_fields(flags_with_integers());
        assert_valid_json(format!("{fields}"));
    }

    pub fn make_fields(flags: WasmFlags) -> Fields {
        Fields {
            list: {
                let list =
                    crate::ir::RcList::new(vec![crate::sb3::VarVal::Float(3.0)], &flags).unwrap();
                *list.length_mutable().borrow_mut() = true;
                list.add_type(IrType::Any);
                list
            },
        }
    }
}

crate::instructions_test!(
    mod test2 for data_deletealloflist {
        fields = super::test::make_fields(flags());
    }
);
