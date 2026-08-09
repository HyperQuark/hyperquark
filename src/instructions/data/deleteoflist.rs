use wasm_encoder::BlockType as WasmBlockType;

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
    fields: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let (list_global, Some(length_global)) = func.registries().lists().register(&fields.list)?
    else {
        hq_bug!("tried to insertatlist of a list with immutable length")
    };
    let array_type = func.registries().lists().array_type(&fields.list)?;
    let index_local = func.local(ValType::I32)?;
    func.free_local(index_local)?;
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
    Ok(Rc::from([IrType::Int]))
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
        let fields = make_fields(IrType::Any, flags_with_integers());
        assert_valid_json(format!("{fields}"));
    }

    pub fn make_fields(ty: IrType, flags: WasmFlags) -> Fields {
        Fields {
            list: make_list(true, ty, flags),
        }
    }
}

crate::instructions_test!(
    mod test_int for data_deleteoflist(t) {
        fields = super::test::make_fields(IrType::Int, flags());
        flags = super::test::flags_with_integers();
    }
);

crate::instructions_test!(
    mod test_float for data_deleteoflist(t) {
        fields = super::test::make_fields(IrType::Float, flags());
    }
);

crate::instructions_test!(
    mod test_string for data_deleteoflist(t) {
        fields = super::test::make_fields(IrType::String, flags());
    }
);

crate::instructions_test!(
    mod test_any for data_deleteoflist(t) {
        fields = super::test::make_fields(IrType::Any, flags());
    }
);
