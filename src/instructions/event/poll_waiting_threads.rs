//! This is a bit of a strange instruction in that it relies on only ever being used inside a step
//! that was spawned from an `event_broadcast_and_wait` block. Any other use will cause invalid
//! WASM to be generated.
//!
//! Returns 1 if still waiting on any threads, 0 otherwise.

use wasm_encoder::BlockType as WasmBlockType;

use super::super::prelude::*;
use crate::wasm::StepFunc;
use crate::wasm::registries::functions::static_functions::DynArrayLen;
use crate::wasm::registries::types::{
    TArray, TConstField, TMutField, TNonNullable, TNullable, TStackArray, TStackStruct, TStruct,
    TType,
};

type TWaitingThreadArray = TArray<TMutField<TNullable<TStackArray>>>;
type TPollStruct = TStruct<(TConstField<TNonNullable<TWaitingThreadArray>>, ())>;

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let types = Rc::clone(func.registries().types());

    let thread_array_type = types.register_comp::<TWaitingThreadArray, _>()?;
    let poll_struct_type = types.register_comp::<TPollStruct, _>()?;

    let arr_local = func.local(<TNonNullable<TWaitingThreadArray>>::ty(&types)?)?;
    func.free_local(arr_local)?;

    let arr_len_local = func.local(ValType::I32)?;
    let i_local = func.local(ValType::I32)?;
    let stack_local = func.local(<TNullable<TStackStruct>>::ty(&types)?)?;
    let wait_local = func.local(ValType::I32)?;
    func.free_local(arr_len_local)?;
    func.free_local(stack_local)?;
    func.free_local(i_local)?;
    func.free_local(wait_local)?;

    let dyn_array_len = func
        .registries()
        .static_functions()
        .register::<DynArrayLen<TNullable<TStackArray>>, _>()?;

    Ok(wasm![
        LocalGet(1), // this step should never have additional function arguments so this is fine
        RefCastNonNull(TPollStruct::ty(&types)?),
        StructGet {
            struct_type_index: poll_struct_type,
            field_index: 0,
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

            LocalGet(arr_local),
            LocalGet(i_local),
            ArrayGet(thread_array_type),
            LocalTee(stack_local),
            RefIsNull,
            BrIf(0),

            LocalGet(stack_local),
            #StaticFunctionCall(dyn_array_len),
            I32Eqz,
            If(WasmBlockType::Empty),
                LocalGet(arr_local),
                LocalGet(i_local),
                RefNull(TStackArray::ty(&types)?),
                Br(1),
            End,

            I32Const(1),
            LocalSet(wait_local),
            Br(1),
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
