//! Provides the logic for having boxed input types to blocks

use super::IrOpcode;
use crate::prelude::*;
use crate::wasm::WasmProject;
use crate::{ir::Type as IrType, wasm::StepFunc};
use itertools::Itertools;
use wasm_encoder::Instruction::{self, *};
use wasm_encoder::{BlockType, RefType, ValType};

/// Canonical NaN + bit 33, + string pointer in bits 1-32
const BOXED_STRING_PATTERN: i64 = 0x7FF80001 << 32;
/// Canonical NaN + bit 33, + i32 in bits 1-32
const BOXED_INT_PATTERN: i64 = 0x7ff80002 << 32;

/// generates branches (or not, if an input is not boxed) for a list of remaining input types.
/// This sort of recursion makes me feel distinctly uneasy; I'm just waiting for a stack
/// overflow.
/// TODO: tail-recursify/loopify?
fn generate_branches(
    func: &StepFunc,
    processed_inputs: &[IrType],
    remaining_inputs: &[(Box<[IrType]>, u32)], // u32 is local index
    opcode: &IrOpcode,
    output_type: Option<IrType>,
) -> HQResult<Vec<Instruction<'static>>> {
    if remaining_inputs.is_empty() {
        hq_assert!(processed_inputs.iter().all(|ty| ty.is_base_type()));
        crate::log(format!("processed_inputs:{:?}", processed_inputs).as_str());
        let mut wasm = opcode.wasm(func, Rc::from(processed_inputs))?;
        // if the overall output is boxed, but this particular branch produces an unboxed result
        // (which i think all branches probably should?), box it.
        // TODO: split this into another function somewhere? it seems like this should
        // be useful somewhere else as well
        if let Some(this_output) = opcode.output_type(Rc::from(processed_inputs))? {
            if this_output.is_base_type()
                && !output_type
                    .ok_or(make_hq_bug!("expected no output type but got one"))?
                    .is_base_type()
            {
                wasm.append(&mut match this_output.base_type().unwrap() {
                    IrType::Float => vec![I64ReinterpretF64],
                    IrType::QuasiInt => vec![I64ExtendI32S, I64Const(BOXED_INT_PATTERN), I64And],
                    IrType::String => {
                        let table_index = func
                            .registries()
                            .tables()
                            .register("strings".into(), (RefType::EXTERNREF, 0))?;
                        let externref_local = func.local(ValType::EXTERNREF)?;
                        vec![
                            LocalSet(externref_local),
                            TableSize(table_index),
                            I32Const(1),
                            TableGrow(table_index),
                            LocalGet(externref_local),
                            TableSet(table_index),
                            I64ExtendI32S,
                            I64Const(BOXED_STRING_PATTERN),
                            I64And,
                        ]
                    }
                    _ => unreachable!(),
                });
            }
        }
        return Ok(wasm);
    }
    let (curr_input, local_idx) = &remaining_inputs[0];
    let local_idx = *local_idx; // variable shadowing feels evil but hey it works
    let mut wasm = vec![LocalGet(local_idx)];
    Ok(if curr_input.len() == 1 {
        let mut processed_inputs = processed_inputs.to_vec();
        processed_inputs.push(curr_input[0]);
        wasm.append(&mut generate_branches(
            func,
            &processed_inputs,
            &remaining_inputs[1..],
            opcode,
            output_type,
        )?);
        wasm
    } else {
        let if_block_type = BlockType::FunctionType(
            func.registries().types().register_default((
                processed_inputs
                    .iter()
                    .cloned() // &T.clone() is *T
                    .map(WasmProject::ir_type_to_wasm)
                    .collect::<HQResult<Vec<_>>>()?,
                if let Some(out_ty) = output_type {
                    vec![WasmProject::ir_type_to_wasm(out_ty)?]
                } else {
                    vec![]
                },
            ))?,
        );
        let possible_types_num = curr_input.len();
        for (i, ty) in curr_input.iter().enumerate() {
            let base = ty.base_type().ok_or(make_hq_bug!("non-base type found"))?;
            wasm.append(&mut if i == 0 {
                match base {
                    IrType::QuasiInt => vec![
                        I64Const(BOXED_INT_PATTERN),
                        I64And,
                        I64Const(BOXED_INT_PATTERN),
                        I64Eq,
                        If(if_block_type),
                        LocalGet(local_idx),
                        I32WrapI64,
                    ],
                    IrType::String => {
                        let table_index = func
                            .registries()
                            .tables()
                            .register("strings".into(), (RefType::EXTERNREF, 0))?;
                        vec![
                            I64Const(BOXED_STRING_PATTERN),
                            I64And,
                            I64Const(BOXED_STRING_PATTERN),
                            I64Eq,
                            If(if_block_type),
                            LocalGet(local_idx),
                            I32WrapI64,
                            TableGet(table_index),
                        ]
                    }
                    // float guaranteed to be last so no need to check
                    _ => unreachable!(),
                }
            } else if i == possible_types_num - 1 {
                match base {
                    IrType::Float => vec![Else, LocalGet(local_idx), F64ReinterpretI64], // float guaranteed to be last so no need to check
                    IrType::QuasiInt => vec![Else, LocalGet(local_idx), I32WrapI64],
                    IrType::String => {
                        let table_index = func
                            .registries()
                            .tables()
                            .register("strings".into(), (RefType::EXTERNREF, 0))?;
                        vec![Else, LocalGet(local_idx), I32WrapI64, TableGet(table_index)]
                    }
                    _ => unreachable!(),
                }
            } else {
                match base {
                    // float guaranteed to be last so no need to check
                    IrType::Float => vec![Else, LocalGet(local_idx), F64ReinterpretI64],
                    IrType::QuasiInt => vec![
                        Else,
                        LocalGet(local_idx),
                        I64Const(BOXED_INT_PATTERN),
                        I64And,
                        I64Const(BOXED_INT_PATTERN),
                        I64Eq,
                        If(if_block_type),
                        LocalGet(local_idx),
                        I32WrapI64,
                    ],
                    IrType::String => {
                        let table_index = func
                            .registries()
                            .tables()
                            .register("strings".into(), (RefType::EXTERNREF, 0))?;
                        vec![
                            Else,
                            LocalGet(local_idx),
                            I64Const(BOXED_STRING_PATTERN),
                            I64And,
                            I64Const(BOXED_STRING_PATTERN),
                            I64Eq,
                            If(if_block_type),
                            LocalGet(local_idx),
                            I32WrapI64,
                            TableGet(table_index),
                        ]
                    }
                    _ => unreachable!(),
                }
            });
            let mut processed_inputs = processed_inputs.to_vec();
            processed_inputs.push(*ty);
            wasm.append(&mut generate_branches(
                func,
                &processed_inputs,
                &remaining_inputs[1..],
                opcode,
                output_type,
            )?)
        }
        wasm.extend(core::iter::repeat_n(
            Instruction::End,
            possible_types_num - 1, // the last else doesn't need an additional `end` instruction
        ));
        wasm
    })
}

pub fn wrap_instruction(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    opcode: IrOpcode,
) -> HQResult<Vec<Instruction<'static>>> {
    // crate::log(
    //     format!(
    //         "wrap_instruction. inputs: {:?}, opcode: {:?}",
    //         inputs, opcode
    //     )
    //     .as_str(),
    // );

    let output = opcode.output_type(Rc::clone(&inputs))?;

    hq_assert!(inputs.len() == opcode.acceptable_inputs().len());

    // possible base types for each input
    let base_types =
        // check for float last of all, because I don't think there's an easy way of checking
        // if something is *not* a canonical NaN with extra bits
        core::iter::repeat([IrType::QuasiInt, IrType::String, IrType::Float].into_iter())
            .take(inputs.len())
            .enumerate()
            .map(|(i, tys)| {
                tys.filter(|ty| inputs[i].intersects(*ty)).map(|ty| ty.and(inputs[i]))
                    .collect::<Box<[_]>>()
            }).collect::<Vec<_>>();

    crate::log(format!("{:?}", base_types).as_str());

    // sanity check; we have at least one possible input type for each input
    hq_assert!(
        !base_types.iter().any(|tys| tys.len() == 0),
        "empty input type for block {:?}",
        opcode
    );

    let locals = inputs
        .iter()
        .map(|ty| func.local(WasmProject::ir_type_to_wasm(*ty)?))
        .collect::<HQResult<Vec<_>>>()?;

    // for now, chuck each input into a local
    // TODO: change this so that only the inputs following the first boxed input are local-ised
    let mut wasm = locals
        .iter()
        .rev()
        .cloned()
        .map(LocalSet)
        .collect::<Vec<_>>();

    wasm.append(&mut generate_branches(
        func,
        &[],
        base_types
            .into_iter()
            .zip_eq(locals.iter().cloned())
            .collect::<Vec<_>>()
            .as_slice(),
        &opcode,
        output,
    )?);
    Ok(wasm)
}

// TODO: test that wasm compiles correctly. How? Not sure yet.
