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
    let block_type = func
        .registries()
        .types()
        .register_default((vec![ValType::I32], vec![]))?;
    Ok(
        wasm![Block(BlockType::FunctionType(block_type)), I32Eqz, BrIf(0)]
            .into_iter()
            .chain(inner_instructions)
            .chain(wasm![End])
            .collect(),
    )
}

pub fn acceptable_inputs(_fields: &Fields) -> Rc<[IrType]> {
    Rc::new([IrType::Boolean])
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<Option<IrType>> {
    Ok(None)
}

// crate::instructions_test! {none; hq__if; @ super::Fields(None)}
