use super::super::prelude::*;
use crate::wasm::{StepFunc, WasmProject};

#[derive(Clone, Debug)]
pub struct Fields(pub usize, pub IrType);

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "arg_index": {},
        "arg_type": {},
    }}"#,
            self.0, self.1
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields(index, ty): &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    hq_assert!(
        WasmProject::ir_type_to_wasm(*ty)?
            == *func.params().get(*index).ok_or_else(|| make_hq_bug!(
                "proc argument index was out of bounds for func params"
            ))?,
        "proc argument type didn't match that of the corresponding function param"
    );
    Ok(wasm![LocalGet((*index).try_into().map_err(
        |_| make_hq_bug!("argument index out of bounds")
    )?)])
}

pub fn acceptable_inputs(_: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>, &Fields(_, ty): &Fields) -> HQResult<Option<IrType>> {
    Ok(Some(ty))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;
