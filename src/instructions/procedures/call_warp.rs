use wasm_encoder::Instruction as WInstruction;

use super::super::prelude::*;
use crate::instructions_test;
use crate::ir::Proc;
use crate::wasm::{StepFunc, WasmProject};

#[derive(Clone, Debug)]
pub struct Fields {
    pub proc: Rc<Proc>,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "proc": {:?}
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
        .map(|ty| func.local(WasmProject::ir_type_to_wasm(*ty)))
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
        func.free_local(local)?;
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

pub const fn const_fold(
    _inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
    _fields: &Fields,
) -> HQResult<ConstFold> {
    Ok(NotFoldable)
}

#[cfg(test)]
mod test {
    use super::super::super::tests::*;
    use super::*;
    use crate::ir::{PartialStep, StepIndex};

    #[test]
    fn fields_display_is_valid_json() {
        let fields = make_fields(&[IrType::Any]);
        assert_valid_json(format!("{fields}"));
    }

    pub fn test_project_setup(
        wasm_proj: &WasmProject,
        flags: crate::wasm::WasmFlags,
        inputs: &[IrType],
    ) {
        let proc_step_func = StepFunc::new_with_types(
            inputs
                .iter()
                .map(|ty| WasmProject::ir_type_to_wasm(*ty))
                .chain([
                    ValType::I32,
                    crate::wasm::registries::TypeRegistry::STRUCT_REF,
                ])
                .collect(),
            vec![].into(),
            Rc::clone(&wasm_proj.registries()),
            flags,
            crate::wasm::StepTarget::Sprite(0),
            0,
            Rc::new(vec![]),
        );
        wasm_proj.steps().borrow_mut().push(proc_step_func);
    }

    pub fn make_fields(inputs: &[IrType]) -> super::Fields {
        super::Fields {
            proc: {
                let target = make_target();
                let proc = Rc::new(super::Proc::new(
                    format!("foo {}", core::iter::repeat_n("%s", inputs.len()).join(" ")).into(),
                    RefCell::new(None),
                    RefCell::new(None),
                    None,
                    false,
                    Box::new([]),
                    Box::new([]),
                    Rc::clone(&target),
                    false,
                ));
                let specific_proc = proc.new_specific_proc();
                *specific_proc.first_step_mut() = PartialStep::Finished(StepIndex(0));
                *proc.warped_specific_proc_mut() = Some(specific_proc);
                proc
            },
        }
    }
}

instructions_test!(
    mod test_nullary for procedures_call_warp {
        fields = super::test::make_fields(&[]);
        setup = |proj, flags| super::test::test_project_setup(proj, flags, &[]);
    }
);

instructions_test!(
    mod test_unary_string for procedures_call_warp(t) {
        fields = super::test::make_fields(&[IrType::String]);
        setup = |proj, flags| super::test::test_project_setup(proj, flags, &[IrType::String]);
    }
);

instructions_test!(
    mod test_unary_int for procedures_call_warp(t) {
        fields = super::test::make_fields(&[IrType::Int]);
        setup = |proj, flags| super::test::test_project_setup(proj, flags, &[IrType::Int]);
    }
);

instructions_test!(
    mod test_unary_float for procedures_call_warp(t) {
        fields = super::test::make_fields(&[IrType::Float]);
        setup = |proj, flags| super::test::test_project_setup(proj, flags, &[IrType::Float]);
    }
);

instructions_test!(
    mod test_unary_bool for procedures_call_warp(t) {
        fields = super::test::make_fields(&[IrType::Boolean]);
        setup = |proj, flags| super::test::test_project_setup(proj, flags, &[IrType::Boolean]);
    }
);

instructions_test!(
    mod test_unary_any for procedures_call_warp(t) {
        fields = super::test::make_fields(&[IrType::Any]);
        setup = |proj, flags| super::test::test_project_setup(proj, flags, &[IrType::Any]);
    }
);
