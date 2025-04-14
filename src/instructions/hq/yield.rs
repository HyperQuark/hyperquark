use super::super::prelude::*;
use crate::ir::Step;
use crate::wasm::{GlobalExportable, GlobalMutable, StepFunc};
use wasm_encoder::{ConstExpr, HeapType};

#[derive(Clone, Debug)]
pub enum YieldMode {
    Tail(Rc<Step>),
    Inline(Rc<Step>),
    Schedule(Weak<Step>),
    None,
}

#[derive(Clone, Debug)]
pub struct Fields {
    pub mode: YieldMode,
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields { mode: yield_mode }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let noop_global = func.registries().globals().register(
        "noop_func".into(),
        (
            ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::Concrete(
                    func.registries()
                        .types()
                        .register_default((vec![ValType::I32], vec![]))?,
                ),
            }),
            ConstExpr::ref_func(0), // this is a placeholder.
            GlobalMutable(false),
            GlobalExportable(false),
        ),
    )?;

    let step_func_ty = func
        .registries()
        .types()
        .register_default((vec![ValType::I32], vec![]))?;
    let threads_table = func.registries().tables().register(
        "threads".into(),
        (
            RefType {
                nullable: false,
                heap_type: HeapType::Concrete(step_func_ty),
            },
            0,
            // this default gets fixed up in src/wasm/tables.rs
            None,
        ),
    )?;

    let threads_count = func.registries().globals().register(
        "threads_count".into(),
        (
            ValType::I32,
            ConstExpr::i32_const(0),
            GlobalMutable(true),
            GlobalExportable(true),
        ),
    )?;

    Ok(match yield_mode {
        YieldMode::None => wasm![
            LocalGet(0),
            GlobalGet(noop_global),
            TableSet(threads_table),
            GlobalGet(threads_count),
            I32Const(1),
            I32Sub,
            GlobalSet(threads_count),
            Return
        ],
        YieldMode::Inline(step) => func.compile_inner_step(Rc::clone(step))?,
        YieldMode::Schedule(weak_step) => {
            let step =
                Weak::upgrade(weak_step).ok_or(make_hq_bug!("couldn't upgrade Weak<Step>"))?;
            step.make_used_non_inline()?;
            wasm![
                LocalGet(0),
                #LazyStepRef(Weak::clone(weak_step)),
                TableSet(threads_table),
                Return
            ]
        }
        _ => hq_todo!(),
    })
}

pub fn acceptable_inputs(_fields: &Fields) -> Rc<[IrType]> {
    Rc::new([])
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    Ok(None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

crate::instructions_test! {none; hq_yield; @ super::Fields { mode: super::YieldMode::None }}
