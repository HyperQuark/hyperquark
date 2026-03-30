//! This is a bit of a strange instruction in that it relies on only ever being used inside a step
//! that was spawned from a `sensing_askandwait` block (for now; there is potential to be expanded
//! to other usecases which is why this is in the hq category rather than sensing). Any other usage
//! will result in invalid wasm.
//!
//! Returns 1 if still waiting on any threads, 0 otherwise.

use wasm_encoder::{FieldType, HeapType, StorageType};

use super::super::prelude::*;
use crate::wasm::StepFunc;

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let i8_struct_type = func.registries().types().struct_(vec![FieldType {
        element_type: StorageType::I8,
        mutable: true,
    }])?;

    Ok(wasm![
        LocalGet(1), // this should never have additional function arguments so this is fine
        RefCastNonNull(HeapType::Concrete(i8_struct_type)),
        StructGetS {
            struct_type_index: i8_struct_type,
            field_index: 0
        },
    ])
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(Singleton(IrType::Boolean))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}
