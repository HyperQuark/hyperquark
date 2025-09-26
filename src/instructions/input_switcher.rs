//! Provides the logic for having boxed input types to blocks

use super::HqCastFields;
use super::IrOpcode;
use super::prelude::*;
use crate::ir::{Type as IrType, base_types};
use crate::wasm::GlobalExportable;
use crate::wasm::GlobalMutable;
use crate::wasm::StepFunc;
use crate::wasm::StringsTable;
use crate::wasm::WasmProject;
use itertools::Itertools;
use wasm_encoder::ConstExpr;
use wasm_encoder::{BlockType, Instruction as WInstruction};
use wasm_gen::wasm;

/// Takes a base type and gives the NaN-boxed pattern for that type
fn boxed_pattern(ty: IrType) -> HQResult<i64> {
    Ok(match ty {
        IrType::QuasiInt => BOXED_INT_PATTERN,
        IrType::String => BOXED_STRING_PATTERN,
        IrType::ColorARGB => BOXED_COLOR_ARGB_PATTERN,
        IrType::ColorRGB => BOXED_COLOR_RGB_PATTERN,
        _ => hq_bug!("bad type for boxed pattern"),
    })
}

fn unbox_instructions(ty: IrType, func: &StepFunc) -> HQResult<Vec<InternalInstruction>> {
    Ok(match ty {
        IrType::QuasiInt | IrType::ColorARGB | IrType::ColorRGB => wasm![I32WrapI64],
        IrType::String => {
            let table_index = func.registries().tables().register::<StringsTable, _>()?;
            wasm![I32WrapI64, TableGet(table_index)]
        }
        IrType::Float => wasm![F64ReinterpretI64],
        _ => hq_bug!("bad type for unboxing instructions"),
    })
}

fn box_type_check(ty: IrType, if_block_type: BlockType) -> HQResult<Vec<InternalInstruction>> {
    Ok(if ty == IrType::Float {
        wasm![]
    } else {
        let box_pattern = boxed_pattern(ty)?;
        wasm![
            I64Const(box_pattern),
            I64And,
            I64Const(box_pattern),
            I64Eq,
            If(if_block_type),
        ]
    })
}

fn cast_instructions(
    pos: usize,
    base: IrType,
    if_block_type: BlockType,
    local_idx: u32,
    func: &StepFunc,
    possible_types_num: usize,
) -> HQResult<Vec<InternalInstruction>> {
    Ok(if pos == 0 {
        box_type_check(base, if_block_type)?
            .into_iter()
            .chain(wasm![LocalGet(local_idx)])
            .chain(unbox_instructions(base, func)?)
            .collect()
    } else if pos == possible_types_num - 1 {
        wasm![Else, LocalGet(local_idx)]
            .into_iter()
            .chain(unbox_instructions(base, func)?)
            .collect()
    } else {
        wasm![Else, LocalGet(local_idx)]
            .into_iter()
            .chain(box_type_check(base, if_block_type)?)
            .chain(wasm![LocalGet(local_idx)])
            .chain(unbox_instructions(base, func)?)
            .collect()
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
    output_type: ReturnType,
) -> HQResult<Vec<InternalInstruction>> {
    if remaining_inputs.is_empty() {
        hq_assert!(processed_inputs.iter().copied().all(IrType::is_base_type));
        let rc_processed_inputs = processed_inputs.into();
        let mut wasm = opcode.wasm(func, Rc::clone(&rc_processed_inputs))?;
        // if the overall output is boxed, but this particular branch produces an unboxed result
        // (which i think all branches probably should?), box it.
        // TODO: split this into another function somewhere? it seems like this should
        // be useful somewhere else as well
        match output_type {
            ReturnType::Singleton(out_ty) => {
                let this_output = opcode.output_type(rc_processed_inputs)?;
                if let ReturnType::Singleton(this_output_ty) = this_output
                    && this_output_ty.is_base_type()
                    && !out_ty.is_base_type()
                {
                    #[expect(clippy::unwrap_used, reason = "asserted that type is base type")]
                    let this_base_type = this_output_ty.base_type().unwrap();
                    wasm.append(&mut wasm![@boxed(this_base_type)]);
                } else if let ReturnType::MultiValue(_) = this_output {
                    crate::warn(
                        "found multi-valued output type for this block in `generate_branches`... suspicious.",
                    );
                }
            }
            ReturnType::None => (),
            ReturnType::MultiValue(_) => {
                crate::warn("found multi-valued output type in `generate_branches`... suspicious.");
            }
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
                match output_type {
                    ReturnType::Singleton(out_ty) => vec![WasmProject::ir_type_to_wasm(out_ty)?],
                    ReturnType::MultiValue(ref out_tys) => out_tys
                        .iter()
                        .copied()
                        .map(WasmProject::ir_type_to_wasm)
                        .collect::<HQResult<_>>()?,
                    ReturnType::None => vec![],
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
                    .singleton_or_else(|| {
                        make_hq_bug!("hq_cast output type was None or multi-valued")
                    })?,
            );
            wasm.append(&mut generate_branches(
                func,
                &vec_processed_inputs,
                &remaining_inputs[1..],
                opcode,
                output_type.clone(),
            )?);
        }
        wasm.extend(core::iter::repeat_n(
            InternalInstruction::Immediate(WInstruction::End),
            possible_types_num - 1, // the last else doesn't need an additional `end` instruction
        ));
    }
    Ok(wasm)
}

pub fn wrap_instruction(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    opcode: &IrOpcode,
) -> HQResult<Vec<InternalInstruction>> {
    if matches!(opcode, &IrOpcode::procedures_call_warp(_)) {
        // we don't want to unbox inputs to procedures, because... reasons
        // TODO: can we carry out monomorphisation on procedures?
        return opcode.wasm(func, inputs);
    }

    hq_assert!(inputs.len() == opcode.acceptable_inputs()?.len());

    // possible base types for each input
    let base_types = base_types(&inputs);

    // sanity check; we have at least one possible input type for each input
    hq_assert!(
        !base_types.iter().any(|tys| tys.is_empty()),
        "empty input type for block {:?}",
        opcode
    );

    let output = opcode.output_type(Rc::clone(&inputs))?;

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

        wasm.append(&mut wasm![I32Const(1), #LazyGlobalSet(refresh_requested),]);
    }
    Ok(wasm)
}
