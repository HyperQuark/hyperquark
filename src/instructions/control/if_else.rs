use wasm_encoder::BlockType;

use super::super::prelude::*;
use crate::ir::Step;

#[derive(Debug, Clone)]
pub struct Fields {
    pub branch_if: Rc<RefCell<Step>>,
    pub branch_else: Rc<RefCell<Step>>,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "branch_if": {},
        "branch_else": {}
    }}"#,
            RefCell::borrow(&self.branch_if),
            RefCell::borrow(&self.branch_else)
        )
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
    let if_instructions = func.compile_inner_step(Rc::clone(branch_if))?;
    let else_instructions = func.compile_inner_step(Rc::clone(branch_else))?;
    let block_type = func
        .registries()
        .types()
        .function(vec![ValType::I32], vec![])?;
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

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([IrType::Boolean]))
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;

pub fn const_fold(
    inputs: &[ConstFoldItem],
    _state: &mut ConstFoldState,
    fields: &Fields,
) -> HQResult<ConstFold> {
    if let ConstFoldItem::Basic(VarVal::Bool(const_condition)) = inputs[0]
        && false
    {
        Ok(ConstFold::Folded(Rc::from([ConstFoldItem::Stack(
            RefCell::borrow(if const_condition {
                &fields.branch_if
            } else {
                &fields.branch_else
            })
            .opcodes()
            .iter()
            .cloned()
            .collect(),
        )])))
    } else {
        Ok(NotFoldable)
    }
}

// crate::instructions_test! {none; hq__if; @ super::Fields(None)}
