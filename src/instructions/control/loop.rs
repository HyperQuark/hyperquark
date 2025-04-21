// for use in warped contexts only.

use super::super::prelude::*;
use crate::ir::Step;
use wasm_encoder::BlockType;

#[derive(Clone, Debug)]
pub struct Fields {
    pub first_condition: Option<Rc<Step>>,
    pub condition: Rc<Step>,
    pub body: Rc<Step>,
    pub flip_if: bool,
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields {
        first_condition,
        condition,
        body,
        flip_if,
    }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let inner_instructions = func.compile_inner_step(Rc::clone(body))?;
    let first_condition_instructions = func.compile_inner_step(Rc::clone(
        &first_condition.clone().unwrap_or(Rc::clone(condition)),
    ))?;
    let condition_instructions = func.compile_inner_step(Rc::clone(condition))?;
    Ok(wasm![Block(BlockType::Empty),]
        .into_iter()
        .chain(first_condition_instructions)
        .chain(if *flip_if {
            wasm![BrIf(0), Loop(BlockType::Empty)]
        } else {
            wasm![I32Eqz, BrIf(0), Loop(BlockType::Empty)]
        })
        .chain(inner_instructions)
        .chain(condition_instructions)
        .chain(if *flip_if {
            wasm![I32Eqz, BrIf(0), End, End]
        } else {
            wasm![BrIf(0), End, End]
        })
        .collect())
}

pub fn acceptable_inputs(_fields: &Fields) -> Rc<[IrType]> {
    Rc::new([])
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    Ok(None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

// crate::instructions_test! {none; hq_repeat; @ super::Fields(None)}
