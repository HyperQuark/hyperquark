use wasm_encoder::{FieldType, HeapType, Instruction as WInstruction, StorageType};

use super::super::prelude::*;
use crate::instructions_test;
use crate::ir::{Proc, StepIndex};
use crate::wasm::registries::functions::static_functions::SpawnThreadInStack;
use crate::wasm::registries::types::TStepFunc;
use crate::wasm::{StepFunc, WasmProject};

#[derive(Clone, Debug)]
pub struct Fields {
    pub proc: Rc<Proc>,
    pub next_step: StepIndex,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "proc": {:?},
        "next_step": {}
    }}"#,
            self.proc.proccode(),
            self.next_step.0,
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    Fields { proc, next_step }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let Some(ref nonwarped_specific_proc) = *proc.nonwarped_specific_proc() else {
        hq_bug!("nonwarped_specific_proc didn't exist for call_nonwarp")
    };

    let arg_struct_type = func.registries().types().struct_(
        (*nonwarped_specific_proc.arg_vars())
            .borrow()
            .iter()
            .map(|var| {
                Ok(FieldType {
                    mutable: false,
                    element_type: StorageType::Val(WasmProject::ir_type_to_wasm(
                        *var.possible_types(),
                    )),
                })
            })
            .collect::<HQResult<Vec<_>>>()?,
    )?;
    let arg_struct_local = func.local(ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(arg_struct_type),
    }))?;
    func.free_local(arg_struct_local)?;

    let spawn_thread_in_stack = func
        .registries()
        .static_functions()
        .register::<SpawnThreadInStack, _>()?;

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
        nonwarped_specific_proc
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
        StructNew(arg_struct_type),
        LocalSet(arg_struct_local),
        LocalGet((func.params().len() - 2).try_into().map_err(|_| make_hq_bug!("local index out of bounds"))?),
        #LazyNonWarpedProcRef(Rc::clone(proc)),
        LocalGet(arg_struct_local),
        #LazyStepRef(*next_step),
        #StaticFunctionCall(spawn_thread_in_stack),
        LocalGet((func.params().len() - 2).try_into().map_err(|_| make_hq_bug!("local index out of bounds"))?),
        LocalGet(arg_struct_local),
        #LazyNonWarpedProcRef(Rc::clone(proc)),
        ReturnCallRef(func.registries().types().register_comp::<TStepFunc, _>()?)
    ]);

    Ok(wasm)
}

pub fn acceptable_inputs(Fields { proc, .. }: &Fields) -> HQResult<Rc<[IrType]>> {
    let Some(ref nonwarped_specific_proc) = *proc.nonwarped_specific_proc() else {
        hq_bug!("nonwarped_specific_proc didn't exist for call_nonwarp")
    };

    Ok(nonwarped_specific_proc
        .arg_vars()
        .try_borrow()?
        .iter()
        .map(|var| *var.possible_types())
        .collect())
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
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
        let fields = make_fields(2);
        assert_valid_json(format!("{fields}"));
    }

    pub fn test_project_setup(wasm_proj: &WasmProject, flags: crate::wasm::WasmFlags) {
        let proc_step_func = StepFunc::new_with_types(
            Box::from([
                ValType::I32,
                crate::wasm::registries::TypeRegistry::STRUCT_REF,
            ]),
            vec![].into(),
            Rc::clone(&wasm_proj.registries()),
            flags,
            crate::wasm::StepTarget::Sprite(0),
            0,
            Rc::new(vec![]),
        );
        wasm_proj.steps().borrow_mut().push(proc_step_func);
    }

    pub fn make_fields(num_inputs: usize) -> super::Fields {
        super::Fields {
            proc: {
                let target = make_target();
                let proc = Rc::new(super::Proc::new(
                    format!("foo {}", core::iter::repeat_n("%s", num_inputs).join(" ")).into(),
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
                *proc.nonwarped_specific_proc_mut() = Some(specific_proc);
                proc
            },
            next_step: super::StepIndex(0),
        }
    }
}

instructions_test!(
    mod test_nullary for procedures_call_nonwarp {
        fields = super::test::make_fields(0);
        setup = super::test::test_project_setup;
    }
);

instructions_test!(
    mod test_unary for procedures_call_nonwarp(t) {
        fields = super::test::make_fields(1);
        setup = super::test::test_project_setup;
    }
);

instructions_test!(
    mod test_ternary for procedures_call_nonwarp(t1, t2, t3) {
        fields = super::test::make_fields(3);
        setup = super::test::test_project_setup;
    }
);
