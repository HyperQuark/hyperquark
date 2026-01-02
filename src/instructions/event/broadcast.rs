use super::super::prelude::*;
use crate::wasm::StepFunc;

#[derive(Clone, Debug)]
pub struct Fields(pub Box<str>);

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "broadcast": "{:}",
    }}"#,
            self.0
        )
    }
}

pub fn wasm(
    _func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields(broadcast): &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    Ok(wasm![
        #LazyBroadcastSpawn(broadcast.clone())
    ])
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
