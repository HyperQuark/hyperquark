use wasm_encoder::{FieldType, HeapType, StorageType};

use super::super::prelude::*;
use crate::ir::StepIndex;
use crate::wasm::StepFunc;
use crate::wasm::registries::functions::static_functions::{MarkWaitingFlag, SpawnThreadInStack};

#[derive(Clone, Debug)]
pub struct Fields {
    pub poll_step: StepIndex,
    pub next_step: StepIndex,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "poll_step": {},
        "next_step": {}
    }}"#,
            self.poll_step.0, self.next_step.0,
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields {
        poll_step,
        next_step,
    }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let i8_struct_type = func.registries().types().struct_(vec![FieldType {
        element_type: StorageType::I8,
        mutable: true,
    }])?;
    let struct_valtype = ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(i8_struct_type),
    });
    let struct_local = func.local(struct_valtype)?;
    func.free_local(struct_local)?;

    let spawn_thread_func = func
        .registries()
        .static_functions()
        .register::<SpawnThreadInStack, _>()?;

    let queue_ask = func.registries().external_functions().register(
        ("sensing", "queue_ask".into()),
        (vec![ValType::EXTERNREF, struct_valtype], vec![]),
    )?;

    // register the exported function that is called by JS,
    // otherwise it won't be registered and thus will be undefined!
    func.registries()
        .static_functions()
        .register::<MarkWaitingFlag, usize>()?;

    Ok(wasm![
        LocalGet(
            (func.params().len() - 2)
                .try_into()
                .map_err(|_| make_hq_bug!("local index out of bounds"))?
        ),
        #LazyStepRef(*poll_step),
        StructNewDefault(i8_struct_type),
        LocalTee(struct_local),
        #LazyStepRef(*next_step),
        #StaticFunctionCall(spawn_thread_func),
        LocalGet(struct_local),
        Call(queue_ask),
    ])
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::String]))
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
    use crate::wasm::registries::TypeRegistry;
    use crate::wasm::{StepTarget, WasmFlags, WasmProject};

    #[test]
    fn fields_display_is_valid_json() {
        let fields = make_fields();
        assert_valid_json(format!("{fields}"));
    }

    pub fn make_fields() -> Fields {
        Fields {
            poll_step: StepIndex(0),
            next_step: StepIndex(0),
        }
    }

    pub fn setup_project(wasm_proj: &WasmProject, flags: WasmFlags) {
        let step_func = StepFunc::new_with_types(
            Box::from([ValType::I32, TypeRegistry::STRUCT_REF]),
            Box::from([]),
            wasm_proj.registries(),
            flags,
            StepTarget::Sprite(0),
            0,
            Rc::new(vec![]),
        );
        wasm_proj.steps().borrow_mut().push(step_func);
    }
}

crate::instructions_test! (
    mod tests for sensing_askandwait(t) {
        fields = super::test::make_fields();
        setup = super::test::setup_project;
    }
);
