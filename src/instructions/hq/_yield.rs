use crate::ir::{Step, Type as IrType};
use crate::prelude::*;
use crate::wasm::{byte_offset, StepFunc};
use wasm_encoder::Instruction::{self, *};
use wasm_encoder::MemArg;

#[derive(Clone, Debug)]
pub struct Fields(pub Option<Rc<Step>>);

pub fn wasm(
    _func: &StepFunc,
    _inputs: Rc<[IrType]>,
    fields: &Fields,
) -> HQResult<Vec<Instruction<'static>>> {
    Ok(if let Some(next_step) = &fields.0 {
        hq_todo!()
    } else {
        vec![
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

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([])
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    Ok(None)
}

crate::instructions_test! {none;@ super::Fields(None)}
