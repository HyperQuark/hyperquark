use super::super::prelude::*;
use crate::ir::Step;
use wasm_encoder::BlockType;

#[derive(Clone, Debug)]
pub struct Fields {
    pub branch_if: Rc<Step>,
    pub branch_else: Rc<Step>,
}
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields {
        branch_if,
        branch_else,
    }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let if_instructions = func.compile_inner_step(branch_if)?;
    let else_instructions = func.compile_inner_step(branch_else)?;
    let block_type = func
        .registries()
        .types()
        .register_default((vec![ValType::I32], vec![]))?;
    Ok(wasm![
        Block(BlockType::FunctionType(block_type)),
        Block(BlockType::FunctionType(block_type)),
        I32Eqz,
        BrIf(0)
    ]
    .into_iter()
    .chain(if_instructions)
    .chain(wasm![Br(1), End,])
    .chain(else_instructions)
    .chain(wasm![End])
    .collect())
}

pub fn acceptable_inputs(_fields: &Fields) -> Rc<[IrType]> {
    Rc::new([IrType::Boolean])
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    Ok(None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

// crate::instructions_test! {none; hq__if; @ super::Fields(None)}
