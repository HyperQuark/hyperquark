use core::result;

use crate::ir::{Step, Type as IrType};
use crate::prelude::*;
use crate::wasm::{byte_offset, StepFunc, WasmProject};
use wasm_encoder::Instruction::{self, *};
use wasm_encoder::{BlockType, MemArg, ValType};

#[derive(Clone, Debug)]
pub struct Fields(pub IrType);

/// Canonical NaN + bit 33, + string pointer in bits 1-32
const BOXED_STRING_PATTERN: i64 = 0x7FF80001 << 32;
/// Canonical NaN + bit 33, + i32 in bits 1-32
const BOXED_INT_PATTERN: i64 = 0x7ff80002 << 32;

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    &Fields(to): &Fields,
) -> HQResult<Vec<Instruction<'static>>> {
    let from = inputs[0];
    // float needs to be the last input type we check, as I don't think there's a direct way of checking
    // if a value is *not* boxed
    let base_types = [IrType::QuasiInt, IrType::String, IrType::Float];
    let possible_input_types = base_types
        .into_iter()
        .filter(|&ty| from.intersects(ty))
        .map(|ty| Ok((ty, cast_instructions(ty, to, func)?)))
        .collect::<HQResult<Vec<_>>>()?;
    Ok(match possible_input_types.len() {
        0 => hq_bug!("empty input type for hq_cast"),
        1 => possible_input_types[0].1.clone(),
        _ => {
            let result_type = WasmProject::ir_type_to_wasm(to)?;
            let box_local = func.local(ValType::I64)?;
            let possible_types_num = possible_input_types.len() - 1;
            [LocalSet(box_local)]
                .into_iter()
                .chain(
                    possible_input_types
                        .into_iter()
                        .enumerate()
                        .map(|(i, (ty, instrs))| {
                            [
                                if i == 0 {
                                    match ty {
                                        IrType::QuasiInt => vec![
                                            LocalGet(box_local),
                                            I64Const(BOXED_INT_PATTERN),
                                            I64And,
                                            I64Const(BOXED_INT_PATTERN),
                                            I64Eq,
                                            If(BlockType::Result(result_type)),
                                        ],
                                        IrType::String => vec![
                                            LocalGet(box_local),
                                            I64Const(BOXED_STRING_PATTERN),
                                            I64And,
                                            I64Const(BOXED_STRING_PATTERN),
                                            I64Eq,
                                            If(BlockType::Result(result_type)),
                                        ],
                                        // float guaranteed to be last so no need to check
                                        _ => unreachable!(),
                                    }
                                } else if i == possible_types_num {
                                    vec![Else]
                                } else {
                                    match ty {
                                        IrType::Float => vec![Else], // float guaranteed to be last so no need to check
                                        IrType::QuasiInt => vec![
                                            Else,
                                            LocalGet(box_local),
                                            I64Const(BOXED_INT_PATTERN),
                                            I64And,
                                            I64Const(BOXED_INT_PATTERN),
                                            I64Eq,
                                            If(BlockType::Result(result_type)),
                                        ],
                                        IrType::String => vec![
                                            Else,
                                            LocalGet(box_local),
                                            I64Const(BOXED_STRING_PATTERN),
                                            I64And,
                                            I64Const(BOXED_STRING_PATTERN),
                                            I64Eq,
                                            If(BlockType::Result(result_type)),
                                        ],
                                        _ => unreachable!(),
                                    }
                                },
                                vec![LocalGet(box_local)],
                                instrs.clone(),
                            ]
                            .into_iter()
                            .flatten()
                        })
                        .flatten(),
                )
                .chain(std::iter::repeat_n(Instruction::End, possible_types_num))
                .collect::<Vec<_>>()
        }
    })
}

fn cast_instructions(
    from: IrType,
    to: IrType,
    func: &StepFunc,
) -> HQResult<Vec<Instruction<'static>>> {
    // `to` and `from` are guaranteed to be a base type (i.e. float, int/bool or string)
    Ok(if IrType::Number.contains(to) {
        // if casting to a number, we always cast to a float. for now.
        // I suppose this doesn't really make sense if we're casting to a bool,
        // but we should never be casting anything to a bool because in general you can't
        // put round blocks in predicate inputs.
        // TODO: consider the exception (<item in list>)
        if IrType::Float.contains(from) {
            vec![]
        } else if IrType::QuasiInt.contains(from) {
            vec![F64ConvertI32S]
        } else if IrType::String.contains(from) {
            let func_index = func.registries().external_functions().register(
                ("cast", "string2float"),
                (vec![ValType::EXTERNREF], vec![ValType::F64]),
            )?;
            vec![Call(func_index)]
        } else {
            hq_todo!("bad cast: {:?} -> number", to)
        }
    } else if IrType::String.contains(to) {
        if IrType::Float.contains(from) {
            let func_index = func.registries().external_functions().register(
                ("cast", "float2string"),
                (vec![ValType::F64], vec![ValType::EXTERNREF]),
            )?;
            vec![Call(func_index)]
        } else if IrType::QuasiInt.contains(from) {
            let func_index = func.registries().external_functions().register(
                ("cast", "int2string"),
                (vec![ValType::I32], vec![ValType::EXTERNREF]),
            )?;
            vec![Call(func_index)]
        } else if IrType::String.contains(from) {
            vec![]
        } else {
            hq_todo!("bad cast: {:?} -> number", to)
        }
    } else {
        hq_todo!("unimplemented cast: {:?} -> {:?}", to, from)
    })
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::Number.or(IrType::String).or(IrType::Boolean)])
}

pub fn output_type(_inputs: Rc<[IrType]>, &Fields(to): &Fields) -> HQResult<Option<IrType>> {
    Ok(Some(to))
}

crate::instructions_test! {float; t @ super::Fields(IrType::Float)}
crate::instructions_test! {string; t @ super::Fields(IrType::String)}
