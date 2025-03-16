use super::super::prelude::*;
use crate::ir::Step;
use crate::wasm::{GlobalExportable, GlobalMutable, StepFunc};
use wasm_encoder::{ConstExpr, HeapType};

#[derive(Clone, Debug)]
pub enum YieldMode {
    Tail,
    Force,
}

#[derive(Clone, Debug)]
pub struct Fields {
    pub step: Option<Rc<Step>>,
    pub mode: YieldMode,
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<Instruction<'static>>> {
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

    Ok(if let Some(_next_step) = &fields.step {
        hq_todo!()
    } else {
        wasm![
            LocalGet(0),
            GlobalGet(noop_global),
            TableSet(threads_table),
            GlobalGet(threads_count),
            I32Const(1),
            I32Sub,
            GlobalSet(threads_count),
            Return
        ]
    })
}

pub fn acceptable_inputs(_fields: &Fields) -> Rc<[IrType]> {
    Rc::new([])
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    Ok(None)
}

crate::instructions_test! {none; hq__yield; @ super::Fields { step: None, mode: super::YieldMode::Tail }}
