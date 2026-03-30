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

// crate::instructions_test! { tests; sensing_askandwait; t @ super::Fields {
//     poll_step: super::StepIndex(0),
//     next_step: super::StepIndex(0),
// }}
