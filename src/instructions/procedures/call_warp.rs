use super::super::prelude::*;
use crate::ir::Proc;
use crate::wasm::{StepFunc, WasmProject};
use wasm_encoder::Instruction as WInstruction;

#[derive(Clone, Debug)]
pub struct Fields {
    pub proc: Rc<Proc>,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "proc": {:?},
    }}"#,
            self.proc.proccode()
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    Fields { proc }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let Some(ref warped_specific_proc) = *proc.warped_specific_proc() else {
        hq_bug!("warped_specific_proc didn't exist for call_warp")
    };

    let locals = inputs
        .iter()
        .map(|ty| func.local(WasmProject::ir_type_to_wasm(*ty)?))
        .collect::<HQResult<Vec<_>>>()?;

    let mut wasm = locals
        .iter()
        .rev()
        .copied()
        .map(WInstruction::LocalSet)
        .map(InternalInstruction::Immediate)
        .collect::<Vec<_>>();

    for ((&input, local), param) in inputs.iter().zip(locals).zip(
        warped_specific_proc
            .arg_vars()
            .try_borrow()?
            .iter()
            .map(|var| **var.possible_types().borrow()),
    ) {
        wasm.extend(if param.is_base_type() {
            wasm![LocalGet(local)]
        } else {
            wasm![
                LocalGet(local),
                @boxed(input),
            ]
        });
    }

    wasm.extend(wasm![
        LocalGet((func.params().len() - 2).try_into().map_err(|_| make_hq_bug!("local index out of bounds"))?),
        LocalGet((func.params().len() - 1).try_into().map_err(|_| make_hq_bug!("local index out of bounds"))?),
        #LazyWarpedProcCall(Rc::clone(proc))
    ]);

    Ok(wasm)
}

pub fn acceptable_inputs(Fields { proc }: &Fields) -> HQResult<Rc<[IrType]>> {
    let Some(ref warped_specific_proc) = *proc.warped_specific_proc() else {
        hq_bug!("warped_specific_proc didn't exist for call_warp")
    };
    Ok(warped_specific_proc
        .arg_vars()
        .try_borrow()?
        .iter()
        .map(|var| *var.possible_types())
        .collect())
}

pub fn output_type(_inputs: Rc<[IrType]>, Fields { proc }: &Fields) -> HQResult<ReturnType> {
    let Some(ref warped_specific_proc) = *proc.warped_specific_proc() else {
        hq_bug!("warped_specific_proc didn't exist for call_warp")
    };
    Ok(MultiValue(
        warped_specific_proc
            .return_vars()
            .try_borrow()?
            .iter()
            .map(|var| *var.possible_types())
            .collect(),
    ))
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;
