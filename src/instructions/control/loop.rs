// for use in warped contexts only.

use wasm_encoder::BlockType;

use super::super::prelude::*;
use crate::ir::Step;

#[derive(Debug, Clone)]
pub struct Fields {
    pub first_condition: Option<Rc<RefCell<Step>>>,
    pub condition: Rc<RefCell<Step>>,
    pub body: Rc<RefCell<Step>>,
    pub pre_body: Option<Rc<RefCell<Step>>>,
    pub flip_if: bool,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{")?;
        if let Some(ref step) = self.first_condition {
            write!(f, r#""first_condition": {},"#, RefCell::borrow(step))?;
        }
        if let Some(ref step) = self.pre_body {
            write!(f, r#""pre_body": {},"#, RefCell::borrow(step))?;
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
        pre_body,
    }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let inner_instructions = func.compile_inner_step(Rc::clone(body))?;
    let first_condition_instructions = func.compile_inner_step(
        first_condition
            .clone()
            .unwrap_or_else(|| Rc::clone(condition)),
    )?;
    let condition_instructions = func.compile_inner_step(Rc::clone(condition))?;
    let pre_body_instructions = pre_body.as_ref().map_or_else(
        || Ok(vec![]),
        |pre_body_step| func.compile_inner_step(Rc::clone(pre_body_step)),
    )?;
    Ok(wasm![Block(BlockType::Empty),]
        .into_iter()
        .chain(first_condition_instructions)
        .chain(if *flip_if {
            wasm![BrIf(0), Loop(BlockType::Empty)]
        } else {
            wasm![I32Eqz, BrIf(0), Loop(BlockType::Empty)]
        })
        .chain(pre_body_instructions)
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

#[cfg(test)]
mod test {
    use super::super::super::tests::*;
    use super::*;
    use crate::ir::StepContext;

    #[test]
    fn fields_display_is_valid_json() {
        for ((fc, pb), fi) in [true, false]
            .into_iter()
            .cartesian_product([true, false])
            .cartesian_product([true, false])
        {
            let fields = make_fields(fc, pb, fi);
            assert_valid_json(format!("{fields}"));
        }
    }

    fn make_condition_step(context: &StepContext) -> Rc<RefCell<Step>> {
        Rc::new(RefCell::new(Step::new(
            None,
            context.clone(),
            vec![crate::instructions::IrOpcode::hq_boolean(
                crate::instructions::HqBooleanFields(true),
            )],
            Weak::new(),
            false,
        )))
    }

    pub fn make_fields(first_condition: bool, pre_body: bool, flip_if: bool) -> Fields {
        let target = make_target();
        let context = make_context(&target);
        Fields {
            first_condition: if first_condition {
                None
            } else {
                Some(make_condition_step(&context))
            },
            condition: make_condition_step(&context),
            pre_body: if pre_body {
                None
            } else {
                Some(make_step(&target))
            },
            body: make_step(&target),
            flip_if,
        }
    }
}

crate::instructions_test! (
mod test_fc_pb_fi for control_loop {
    fields = super::test::make_fields(true, true, true);
}
);

crate::instructions_test! (
mod test_fc_pb for control_loop {
    fields = super::test::make_fields(true, true, false);
}
);

crate::instructions_test! (
mod test_fc_fi for control_loop {
    fields = super::test::make_fields(true, false, true);
}
);

crate::instructions_test! (
mod test_fc for control_loop {
    fields = super::test::make_fields(true, false, false);
}
);

crate::instructions_test! (
mod test_pb_fi for control_loop {
    fields = super::test::make_fields(false, true, true);
}
);

crate::instructions_test! (
mod test_pb for control_loop {
    fields = super::test::make_fields(false, true, false);
}
);

crate::instructions_test! (
mod test_fi for control_loop {
    fields = super::test::make_fields(false, false, true);
}
);

crate::instructions_test! (
mod test_defaults for control_loop {
    fields = super::test::make_fields(false, false, false);
}
);
