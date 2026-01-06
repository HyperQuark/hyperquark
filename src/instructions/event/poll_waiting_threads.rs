//! This is a bit of a strange instruction in that it relies on only ever being used inside a step
//! that was spawned from an `event_broadcast_and_wait` block. Any other use will cause invalid
//! WASM to be generated.
//!
//! Returns 1 if still waiting on any threads, 0 otherwise.

use wasm_encoder::{BlockType as WasmBlockType, FieldType, HeapType, StorageType};

use super::super::prelude::*;
use crate::wasm::{StepFunc, ThreadsTable};

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let i32_array_type = func
        .registries()
        .types()
        .array(StorageType::Val(ValType::I32), true)?;
    let poll_struct_type = func.registries().types().struct_(vec![FieldType {
        mutable: false,
        element_type: StorageType::Val(ValType::Ref(RefType {
            nullable: false,
            heap_type: HeapType::Concrete(i32_array_type),
        })),
    }])?;

    let arr_local = func.local(ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(i32_array_type),
    }))?;

    let arr_len_local = func.local(ValType::I32)?;
    let i_local = func.local(ValType::I32)?;
    let wait_local = func.local(ValType::I32)?;

    let threads_table = func.registries().tables().register::<ThreadsTable, _>()?;

    Ok(wasm![
        LocalGet(1), // this should never have additional function arguments so this is fine
        RefCastNonNull(HeapType::Concrete(poll_struct_type)),
        StructGet {
            struct_type_index: poll_struct_type,
            field_index: 0
        },
        LocalTee(arr_local),
        ArrayLen,
        LocalSet(arr_len_local),
        I32Const(-1),
        LocalSet(i_local),
        I32Const(0),
        LocalSet(wait_local),
        Block(WasmBlockType::Empty),
        Loop(WasmBlockType::Empty),
        LocalGet(i_local),
        I32Const(1),
        I32Add,
        LocalTee(i_local),
        LocalGet(arr_len_local),
        I32Eq,
        BrIf(1),
        LocalGet(i_local),
        I32Const(0),
        I32LtS,
        BrIf(0),
        Block(WasmBlockType::Empty),
        LocalGet(arr_local),
        LocalGet(i_local),
        ArrayGet(i32_array_type),
        TableGet(threads_table),
        RefIsNull,
        BrIf(0),
        I32Const(1),
        LocalSet(wait_local),
        Br(1),
        End,
        LocalGet(arr_local),
        LocalGet(i_local),
        I32Const(-1),
        ArraySet(i32_array_type),
        Br(0),
        End,
        End,
        LocalGet(wait_local),
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
