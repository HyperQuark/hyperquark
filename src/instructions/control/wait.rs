use wasm_encoder::{AbstractHeapType, ConstExpr, FieldType, HeapType, StorageType};

use super::super::prelude::*;
use crate::ir::Step;
use crate::wasm::registries::functions::static_functions::SpawnThreadInStack;
use crate::wasm::{GlobalExportable, GlobalMutable, StepFunc};

#[derive(Clone, Debug)]
pub struct Fields {
    pub poll_step: Rc<Step>,
    pub next_step: Rc<Step>,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "poll_step": {},
        "next_step": {}
    }}"#,
            self.poll_step, self.next_step,
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    Fields {
        poll_step,
        next_step,
    }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let t1 = inputs[0];

    let spawn_thread_in_stack_func = func
        .registries()
        .static_functions()
        .register::<SpawnThreadInStack, _>()?;

    let struct_type = func.registries().types().struct_(vec![FieldType {
        element_type: StorageType::Val(ValType::F64),
        mutable: false,
    }])?;

    let struct_local = func.local(ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(struct_type),
    }))?;

    let timer_global_index = func.registries().globals().register(
        "sensing_timer".into(),
        (
            ValType::F64,
            ConstExpr::f64_const(0.0.into()),
            GlobalMutable(true),
            GlobalExportable(true),
        ),
    )?;

    Ok(
        if t1.contains(IrType::FloatNeg) {
            wasm![
                F64Abs,
            ]
        } else {
            vec![]
        }
        .into_iter()
        .chain(
            wasm![
                #LazyGlobalGet(timer_global_index),
                F64Add,
                StructNew(struct_type),
                LocalSet(struct_local),
                LocalGet((func.params().len() - 2).try_into().map_err(|_| make_hq_bug!("local index out of bounds"))?),
                #LazyStepRef(Rc::downgrade(poll_step)),
                LocalGet(struct_local),
                RefCastNullable(HeapType::Abstract { shared: false, ty: AbstractHeapType::Struct }),
                #LazyStepRef(Rc::downgrade(next_step)),
                #StaticFunctionCall(spawn_thread_in_stack_func),
            ]
        ).collect()
    )
}

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Float]))
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
