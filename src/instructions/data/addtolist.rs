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
    let local = func.local(WasmProject::ir_type_to_wasm(*list.possible_types()))?;
    func.free_local(local)?;
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

    mod test_int for data_addtolist(t) {
        fields = super::test::make_fields(true, IrType::Int, flags());
        flags = super::test::flags_with_integers();
    }

);
crate::instructions_test!(
    mod test_float for data_addtolist(t) {
        fields = super::test::make_fields(true, IrType::Float, flags());
    }
);

crate::instructions_test!(
    mod test_string for data_addtolist(t) {
        fields = super::test::make_fields(true, IrType::String, flags());
    }
);
crate::instructions_test!(
    mod test_any for data_addtolist(t) {
        fields = super::test::make_fields(true, IrType::String, flags());
    }
);
