use super::super::prelude::*;
use crate::ir::Step;
use crate::wasm::{byte_offset, StepFunc};
use wasm_encoder::MemArg;

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
    _func: &StepFunc,
    _inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<Instruction<'static>>> {
    #[allow(unused_variables)]
    Ok(if let Some(next_step) = &fields.step {
        hq_todo!()
    } else {
        wasm![
            LocalGet(0),
            I32Const(byte_offset::THREADS),
            I32Add, // destination (= current thread pos in memory)
            LocalGet(0),
            I32Const(byte_offset::THREADS + 4),
            I32Add, // source (= current thread pos + 4)
            I32Const(0),
            I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                align: 2,
                memory_index: 0,
            }),
            I32Const(4),
            I32Mul,
            LocalGet(0),
            I32Sub, // length (threadnum * 4 - current thread pos)
            MemoryCopy {
                src_mem: 0,
                dst_mem: 0,
            },
            I32Const(0),
            I32Const(0),
            I32Load(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                align: 2,
                memory_index: 0,
            }),
            I32Const(1),
            I32Sub,
            I32Store(MemArg {
                offset: byte_offset::THREAD_NUM
                    .try_into()
                    .map_err(|_| make_hq_bug!("THREAD_NUM out of bounds"))?,
                align: 2,
                memory_index: 0,
            }),
            I32Const(0),
            Return,
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
