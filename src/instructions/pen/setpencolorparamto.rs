use super::super::prelude::*;
use crate::instructions::{HqTextFields, IrOpcode};
use crate::wasm::{StepTarget, mem_layout, registries};
use registries::functions::static_functions::UpdatePenColorFromHSV;
use wasm_encoder::{BlockType as WasmBlockType, MemArg};

pub fn wasm(func: &StepFunc, _inputs: Rc<[IrType]>) -> HQResult<Vec<InternalInstruction>> {
    let StepTarget::Sprite(wasm_target_index) = func.target() else {
        hq_bad_proj!("looks_setsizeto called in stage")
    };
    let mem_pos =
        mem_layout::stage::BLOCK_SIZE + wasm_target_index * mem_layout::sprite::BLOCK_SIZE;
    let hsv2rgb_func = func
        .registries()
        .static_functions()
        .register::<UpdatePenColorFromHSV, _>()?;
    let param_local = func.local(ValType::EXTERNREF)?;
    let value_local = func.local(ValType::F32)?;
    Ok(
        wasm![F32DemoteF64, LocalSet(value_local), LocalTee(param_local),]
            .into_iter()
            .chain(IrOpcode::hq_text(HqTextFields("color".into())).wasm(func, Rc::from([]))?)
            .chain(
                IrOpcode::operator_equals
                    .wasm(func, Rc::from([IrType::String, IrType::StringNan]))?,
            )
            .chain(wasm![
                If(WasmBlockType::Empty),
                I32Const(0),
                LocalGet(value_local),
                F32Store(MemArg {
                    offset: (mem_pos + mem_layout::sprite::PEN_COLOR).into(),
                    align: 2,
                    memory_index: 0
                }),
                Else,
                LocalGet(param_local),
            ])
            .chain(IrOpcode::hq_text(HqTextFields("saturation".into())).wasm(func, Rc::from([]))?)
            .chain(
                IrOpcode::operator_equals
                    .wasm(func, Rc::from([IrType::String, IrType::StringNan]))?,
            )
            .chain(wasm![
                If(WasmBlockType::Empty),
                I32Const(0),
                LocalGet(value_local),
                F32Store(MemArg {
                    offset: (mem_pos + mem_layout::sprite::PEN_SATURATION).into(),
                    align: 2,
                    memory_index: 0
                }),
                Else,
                LocalGet(param_local),
            ])
            .chain(IrOpcode::hq_text(HqTextFields("brightness".into())).wasm(func, Rc::from([]))?)
            .chain(
                IrOpcode::operator_equals
                    .wasm(func, Rc::from([IrType::String, IrType::StringNan]))?,
            )
            .chain(wasm![
                If(WasmBlockType::Empty),
                I32Const(0),
                LocalGet(value_local),
                F32Store(MemArg {
                    offset: (mem_pos + mem_layout::sprite::PEN_BRIGHTNESS).into(),
                    align: 2,
                    memory_index: 0
                }),
                Else,
                LocalGet(param_local),
            ])
            .chain(IrOpcode::hq_text(HqTextFields("transparency".into())).wasm(func, Rc::from([]))?)
            .chain(
                IrOpcode::operator_equals
                    .wasm(func, Rc::from([IrType::String, IrType::StringNan]))?,
            )
            .chain(wasm![
                If(WasmBlockType::Empty),
                I32Const(0),
                LocalGet(value_local),
                F32Store(MemArg {
                    offset: (mem_pos + mem_layout::sprite::PEN_TRANSPARENCY).into(),
                    align: 2,
                    memory_index: 0
                }),
                End,
                End,
                End,
                End,
                I32Const(
                    wasm_target_index
                        .try_into()
                        .map_err(|_| make_hq_bug!("target index out of bounds"))?
                ),
                #StaticFunctionCall(hsv2rgb_func),
            ])
            .collect(),
    )
}

pub fn acceptable_inputs() -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::String, IrType::Float]))
}

pub fn output_type(_inputs: Rc<[IrType]>) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {tests_colour; pen_setpencolorparamto; t1, t2 }
