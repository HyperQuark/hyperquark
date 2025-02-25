use super::super::prelude::*;
use crate::ir::Step;
use wasm_encoder::BlockType;

#[derive(Clone, Debug)]
pub struct Fields(pub Rc<Step>);

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields(body): &Fields,
) -> HQResult<Vec<Instruction<'static>>> {
    let inner_instructions = func.compile_inner_step(Rc::clone(body))?;
    let times_local = func.local(ValType::I32)?;
    let counter_local = func.local(ValType::I32)?;
    Ok(wasm![
        LocalSet(times_local),
        I32Const(0),
        LocalSet(counter_local),
        Block(BlockType::Empty),
        Loop(BlockType::Empty),
        LocalGet(counter_local),
        LocalGet(times_local),
        I32GeS,
        BrIf(1),
        LocalGet(counter_local),
        I32Const(1),
        I32Add,
        LocalSet(counter_local)
    ]
    .into_iter()
    .chain(inner_instructions)
    .chain(wasm![Br(0), End, End])
    .collect())
}

pub fn acceptable_inputs() -> Rc<[IrType]> {
    Rc::new([IrType::Int])
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    Ok(None)
}

// crate::instructions_test! {none; hq_repeat; @ super::Fields(None)}
