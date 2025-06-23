//! Provides the logic for having boxed input types to blocks

use super::prelude::*;
use super::HqCastFields;
use super::IrOpcode;
use crate::wasm::GlobalExportable;
use crate::wasm::GlobalMutable;
use crate::wasm::TableOptions;
use crate::wasm::WasmProject;
use crate::{ir::Type as IrType, wasm::StepFunc};
use itertools::Itertools;
use wasm_encoder::ConstExpr;
use wasm_encoder::{BlockType, Instruction as WInstruction, RefType};
use wasm_gen::wasm;

fn cast_instructions(
    pos: usize,
    base: IrType,
    if_block_type: BlockType,
    local_idx: u32,
    func: &StepFunc,
    possible_types_num: usize,
) -> HQResult<Vec<InternalInstruction>> {
    Ok(if pos == 0 {
        match base {
            IrType::QuasiInt => wasm![
                I64Const(BOXED_INT_PATTERN),
                I64And,
                I64Const(BOXED_INT_PATTERN),
                I64Eq,
                If(if_block_type),
                LocalGet(local_idx),
                I32WrapI64,
            ],
            IrType::String => {
                let table_index = func.registries().tables().register(
                    "strings".into(),
                    TableOptions {
                        element_type: RefType::EXTERNREF,
                        min: 0,
                        // TODO: use js string imports for preknown strings
                        max: None,
                        init: None,
                    },
                )?;
                wasm![
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
    } else if pos == possible_types_num - 1 {
        match base {
            IrType::Float => wasm![Else, LocalGet(local_idx), F64ReinterpretI64], // float guaranteed to be last so no need to check
            IrType::QuasiInt => wasm![Else, LocalGet(local_idx), I32WrapI64],
            IrType::String => {
                let table_index = func.registries().tables().register(
                    "strings".into(),
                    TableOptions {
                        element_type: RefType::EXTERNREF,
                        min: 0,
                        // TODO: use js string imports for preknown strings
                        max: None,
                        init: None,
                    },
                )?;
                wasm![Else, LocalGet(local_idx), I32WrapI64, TableGet(table_index)]
            }
            _ => unreachable!(),
        }
    } else {
        match base {
            // float guaranteed to be last so no need to check
            IrType::Float => wasm![Else, LocalGet(local_idx), F64ReinterpretI64],
            IrType::QuasiInt => wasm![
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
                let table_index = func.registries().tables().register(
                    "strings".into(),
                    TableOptions {
                        element_type: RefType::EXTERNREF,
                        min: 0,
                        max: None,
                        init: None,
                    },
                )?;
                wasm![
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
    })
}

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
) -> HQResult<Vec<InternalInstruction>> {
    if remaining_inputs.is_empty() {
        hq_assert!(processed_inputs.iter().copied().all(IrType::is_base_type));
        let rc_processed_inputs = processed_inputs.into();
        let mut wasm = opcode.wasm(func, Rc::clone(&rc_processed_inputs))?;
        // if the overall output is boxed, but this particular branch produces an unboxed result
        // (which i think all branches probably should?), box it.
        // TODO: split this into another function somewhere? it seems like this should
        // be useful somewhere else as well
        if let Some(this_output) = opcode.output_type(rc_processed_inputs)?
            && this_output.is_base_type()
            && !output_type
                .ok_or_else(|| make_hq_bug!("expected no output type but got one"))?
                .is_base_type()
        {
            #[expect(clippy::unwrap_used, reason = "asserted that type is base type")]
            let this_base_type = this_output.base_type().unwrap();
            wasm.append(&mut wasm![@boxed(this_base_type)]);
        }
        return Ok(wasm);
    }
    let (curr_input, local_idx) = &remaining_inputs[0];
    let local_idx = *local_idx; // variable shadowing feels evil but hey it works
    let mut wasm = wasm![LocalGet(local_idx)];
    if curr_input.len() == 1 {
        let mut vec_processed_inputs = processed_inputs.to_vec();
        vec_processed_inputs.push(curr_input[0]);
        wasm.append(&mut generate_branches(
            func,
            &vec_processed_inputs,
            &remaining_inputs[1..],
            opcode,
            output_type,
        )?);
    } else {
        let if_block_type = BlockType::FunctionType(
            func.registries().types().register_default((
                processed_inputs
                    .iter()
                    .copied()
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
        let allowed_input_types = opcode.acceptable_inputs()?[processed_inputs.len()];
        for (i, ty) in curr_input.iter().enumerate() {
            let base = ty
                .base_type()
                .ok_or_else(|| make_hq_bug!("non-base type found"))?;
            wasm.append(&mut cast_instructions(
                i,
                base,
                if_block_type,
                local_idx,
                func,
                possible_types_num,
            )?);
            if !allowed_input_types.base_types().any(|ty| ty == base) {
                wasm.append(
                    &mut IrOpcode::hq_cast(HqCastFields(allowed_input_types))
                        .wasm(func, Rc::from([*ty]))?,
                );
            }
            let mut vec_processed_inputs = processed_inputs.to_vec();
            vec_processed_inputs.push(
                IrOpcode::hq_cast(HqCastFields(allowed_input_types))
                    .output_type(Rc::from([*ty]))?
                    .ok_or_else(|| make_hq_bug!("hq_cast output type was None"))?,
            );
            wasm.append(&mut generate_branches(
                func,
                &vec_processed_inputs,
                &remaining_inputs[1..],
                opcode,
                output_type,
            )?);
        }
        wasm.extend(core::iter::repeat_n(
            InternalInstruction::Immediate(WInstruction::End),
            possible_types_num - 1, // the last else doesn't need an additional `end` instruction
        ));
    }
    Ok(wasm)
}

#[expect(
    clippy::needless_pass_by_value,
    reason = "passing an Rc by reference doesn't make much sense a lot of the time"
)]
pub fn wrap_instruction(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    opcode: &IrOpcode,
) -> HQResult<Vec<InternalInstruction>> {
    if matches!(opcode, &IrOpcode::procedures_call_warp(_)) {
        // we don't want to unbox inputs to procedures, because... reasons
        // TODO: can we carry out monomorphisation on procedures?
        return opcode.wasm(func, inputs)
    }

    let output = opcode.output_type(Rc::clone(&inputs))?;

    hq_assert!(inputs.len() == opcode.acceptable_inputs()?.len());

    // possible base types for each input
    let base_types =
        // check for float last of all, because I don't think there's an easy way of checking
        // if something is *not* a canonical NaN with extra bits
        core::iter::repeat_n([IrType::QuasiInt, IrType::String, IrType::Float].into_iter(), inputs.len())
            .enumerate()
            .map(|(i, tys)| {
                tys.filter(|ty| inputs[i].intersects(*ty)).map(|ty| ty.and(inputs[i]))
                    .collect::<Box<[_]>>()
            }).collect::<Vec<_>>();

    // sanity check; we have at least one possible input type for each input
    hq_assert!(
        !base_types.iter().any(|tys| tys.is_empty()),
        "empty input type for block {:?}",
        opcode
    );

    let locals = inputs
        .iter()
        .map(|ty| func.local(WasmProject::ir_type_to_wasm(*ty)?))
        .collect::<HQResult<Vec<_>>>()?;

    // for now, chuck each input into a local
    // TODO: change this so that only the inputs following the first boxed input are local-ised
    // ...or should we just let wasm-opt deal with this?
    let mut wasm = locals
        .iter()
        .rev()
        .copied()
        .map(WInstruction::LocalSet)
        .map(InternalInstruction::Immediate)
        .collect::<Vec<_>>();

    wasm.append(&mut generate_branches(
        func,
        &[],
        base_types
            .into_iter()
            .zip_eq(locals.iter().copied())
            .collect::<Vec<_>>()
            .as_slice(),
        opcode,
        output,
    )?);
    if opcode.requests_screen_refresh() {
        let refresh_requested = func.registries().globals().register(
            "requests_refresh".into(),
            (
                ValType::I32,
                ConstExpr::i32_const(0),
                GlobalMutable(true),
                GlobalExportable(true),
            ),
        )?;

        wasm.append(&mut wasm![I32Const(1), GlobalSet(refresh_requested),]);
    }
    Ok(wasm)
}
