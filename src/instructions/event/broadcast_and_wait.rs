use wasm_encoder::{HeapType, StorageType};

use super::super::prelude::*;
use crate::instructions_test;
use crate::ir::StepIndex;
use crate::wasm::StepFunc;

#[derive(Clone, Debug)]
pub struct Fields {
    pub broadcast: Box<str>,
    pub poll_step: StepIndex,
    pub next_step: StepIndex,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "broadcast": "{}",
        "poll_step": {},
        "next_step": {}
    }}"#,
            self.broadcast, self.poll_step.0, self.next_step.0,
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields {
        broadcast,
        poll_step,
        next_step,
    }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let i32_array_type = func
        .registries()
        .types()
        .array(StorageType::Val(ValType::I32), true)?;
    let arr_local = func.local(ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(i32_array_type),
    }))?;
    func.free_local(arr_local)?;

    Ok(wasm![
        LocalGet((func.params().len() - 2).try_into().map_err(|_| make_hq_bug!("local index out of bounds"))?),
        #LazyBroadcastSpawnAndWait((broadcast.clone(), *poll_step, *next_step, arr_local))
    ])
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
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
    use super::*;
    use crate::instructions::tests::assert_valid_json;
    use crate::wasm::registries::TypeRegistry;
    use crate::wasm::{StepTarget, WasmFlags, WasmProject};

    #[test]
    fn fields_display_is_valid_json() {
        let fields = make_fields();
        assert_valid_json(format!("{fields}"));
    }

    pub fn make_fields() -> Fields {
        Fields {
            broadcast: "".into(),
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

instructions_test!(
    mod test2 for event_broadcast_and_wait {
        fields = super::test::make_fields();
        setup = super::test::setup_project;
    }
);
