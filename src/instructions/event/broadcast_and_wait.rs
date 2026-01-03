use wasm_encoder::{HeapType, StorageType};

use super::super::prelude::*;
use crate::ir::Step;
use crate::wasm::StepFunc;

#[derive(Clone, Debug)]
pub struct Fields {
    pub broadcast: Box<str>,
    pub poll_step: Rc<Step>,
    pub next_step: Rc<Step>,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "broadcast": "{}",
        "poll_step": {},
        "next_step": {}
    }}"#,
            self.broadcast, self.poll_step, self.next_step,
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields {
        broadcast,
        poll_step,
        next_step,
    }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let i32_array_type = func
        .registries()
        .types()
        .array(StorageType::Val(ValType::I32), true)?;
    let arr_local = func.local(ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(i32_array_type),
    }))?;

    Ok(wasm![
        LocalGet((func.params().len() - 2).try_into().map_err(|_| make_hq_bug!("local index out of bounds"))?),
        #LazyBroadcastSpawnAndWait((broadcast.clone(), Rc::clone(poll_step), Rc::clone(next_step), arr_local))
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
