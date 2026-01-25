// for use in warped contexts only.

use wasm_encoder::BlockType;

use super::super::prelude::*;
use crate::ir::Step;

#[derive(Debug, Clone)]
pub struct Fields {
    pub first_condition: Option<Rc<RefCell<Step>>>,
    pub condition: Rc<RefCell<Step>>,
    pub body: Rc<RefCell<Step>>,
    pub flip_if: bool,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        if let Some(ref step) = self.first_condition {
            write!(f, r#""first_condition": {},"#, RefCell::borrow(step))?;
        }
        write!(
            f,
            r#"
        "condition": {},
        "body": {},
        "flip_if": {}
    }}"#,
            RefCell::borrow(&self.condition),
            RefCell::borrow(&self.body),
            self.flip_if
        )
    }
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
    let first_condition_instructions = func.compile_inner_step(
        first_condition
            .clone()
            .unwrap_or_else(|| Rc::clone(condition)),
    )?;
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

pub fn acceptable_inputs(_fields: &Fields) -> HQResult<Rc<[IrType]>> {
    Ok(Rc::from([]))
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
