use wasm_encoder::{FieldType, HeapType, StorageType};

use super::super::prelude::*;

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let struct_type_index = func.registries().types().struct_(vec![FieldType {
        element_type: StorageType::Val(ValType::F64),
        mutable: false,
    }])?;

    Ok(wasm![
        LocalGet(1),
        RefCastNonNull(HeapType::Concrete(struct_type_index)),
        StructGet {
            struct_type_index,
            field_index: 0
        }
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(Singleton(IrType::FloatPos))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}
