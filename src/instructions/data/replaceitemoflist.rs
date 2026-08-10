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
    fields: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    hq_assert!(inputs.len() == 2);
    let t1 = inputs[0];
    let t2 = inputs[1];
    let (list_global, maybe_length_global) = func.registries().lists().register(&fields.list)?;
    let array_type = func.registries().lists().array_type(&fields.list)?;
    let index_local = func.local(WasmProject::ir_type_to_wasm(t1))?;
    let val_local = func.local(WasmProject::ir_type_to_wasm(t2))?;
    func.free_local(index_local)?;
    func.free_local(val_local)?;
    Ok(wasm![
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
        I32Const(1),
        I32Sub,
        LocalGet(val_local),
    ])
    .chain(if fields.list.possible_types().is_base_type() {
        vec![]
    } else {
        wasm![@boxed(t2)]
    })
    .chain(wasm![ArraySet(array_type), End,])
    .collect())
}

pub fn acceptable_inputs(Fields { list }: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Int, *list.possible_types()]))
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
        let fields = make_fields(true, IrType::Any, flags_with_integers());
        assert_valid_json(format!("{fields}"));
    }

    pub fn make_fields(mutable: bool, ty: IrType, flags: WasmFlags) -> Fields {
        Fields {
            list: make_list(mutable, ty, flags),
        }
    }
}

crate::instructions_test!(
    mod test_int_mut for data_replaceitemoflist(t1, t2) {
        fields = super::test::make_fields(true, IrType::Int, flags());
        flags = super::test::flags_with_integers();
    }
);

crate::instructions_test!(
    mod test_float_mut for data_replaceitemoflist(t1, t2) {
        fields = super::test::make_fields(true, IrType::Float, flags());
    }
);

crate::instructions_test!(
    mod test_string_mut for data_replaceitemoflist(t1, t2) {
        fields = super::test::make_fields(true, IrType::String, flags());
    }
);

crate::instructions_test!(
    mod test_any_mut for data_replaceitemoflist(t1, t2) {
        fields = super::test::make_fields(true, IrType::Any, flags());
    }
);

crate::instructions_test!(
    mod test_int_static for data_replaceitemoflist(t1, t2) {
        fields = super::test::make_fields(false, IrType::Int, flags());
        flags = super::test::flags_with_integers();
    }
);

crate::instructions_test!(
    mod test_float_static for data_replaceitemoflist(t1, t2) {
        fields = super::test::make_fields(false, IrType::Float, flags());
    }
);

crate::instructions_test!(
    mod test_string_static for data_replaceitemoflist(t1, t2) {
        fields = super::test::make_fields(false, IrType::String, flags());
    }
);

crate::instructions_test!(
    mod test_any_static for data_replaceitemoflist(t1, t2) {
        fields = super::test::make_fields(false, IrType::Any, flags());
    }
);
