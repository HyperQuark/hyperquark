use wasm_encoder::{BlockType, HeapType};

use super::super::prelude::*;
use crate::instructions_test;
use crate::ir::{Step, StepIndex};
use crate::wasm::StepFunc;
use crate::wasm::registries::types::TStepFunc;

#[derive(Debug, Clone)]
pub enum YieldMode {
    Inline(Rc<RefCell<Step>>),
    Schedule(StepIndex),
    None,
    Return,
}

impl fmt::Display for YieldMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "mode": {:?}"#,
            match self {
                Self::Inline(_) => "inline",
                Self::Schedule(_) => "schedule",
                Self::None => "none",
                Self::Return => "return",
            }
        )?;
        match self {
            Self::Inline(step) => {
                write!(f, r#", "step": {}"#, RefCell::borrow(step))?;
            }
            Self::Schedule(step) => {
                write!(f, r#", "step_index": {}"#, step.0)?;
            }
            Self::None | Self::Return => (),
        }
        write!(f, "}}")
    }
}

#[derive(Clone, Debug)]
pub struct Fields {
    pub mode: YieldMode,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.mode, f)
    }
}

pub fn wasm(
    func: &StepFunc,
    _inputs: Rc<[IrType]>,
    Fields { mode: yield_mode }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let threads_count = func.registries().globals().threads_count()?;

    Ok(match yield_mode {
        YieldMode::None => {
            let threads_table = func
                .registries()
                .tables()
                .threads_table(func.target(), func.registries().types())?;
            let thread_struct_ty = func.registries().types().thread_struct_type()?;
            let stack_array_ty = func.registries().types().stack_array_type()?;
            let stack_struct_ty = func.registries().types().stack_struct_type()?;
            let thread_struct_local = func.local(ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::Concrete(thread_struct_ty),
            }))?;
            let stack_struct_local = func.local(ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::Concrete(stack_struct_ty),
            }))?;
            let i32_local = func.local(ValType::I32)?;
            let step_func_ty = func.registries().types().register_comp::<TStepFunc, _>()?;
            func.free_local(thread_struct_local)?;
            func.free_local(stack_struct_local)?;
            func.free_local(i32_local)?;

            wasm![
                LocalGet(0),
                TableGet(threads_table),
                RefAsNonNull,
                LocalTee(thread_struct_local),
                StructGet { struct_type_index: thread_struct_ty, field_index: 0 },
                I32Const(1),
                I32Sub,
                LocalTee(i32_local),
                I32Eqz,
                If(BlockType::Empty),
                #LazyGlobalGet(threads_count),
                I32Const(1),
                I32Sub,
                #LazyGlobalSet(threads_count),
                LocalGet(0),
                RefNull(HeapType::Concrete(thread_struct_ty)),
                TableSet(threads_table),
                Return,
                Else,
                LocalGet(thread_struct_local),
                LocalGet(i32_local),
                StructSet {
                    struct_type_index: thread_struct_ty,
                    field_index: 0,
                },
                LocalGet(thread_struct_local),
                StructGet {
                    struct_type_index: thread_struct_ty,
                    field_index: 1,
                },
                LocalGet(i32_local),
                I32Const(1),
                I32Sub,
                ArrayGet(stack_array_ty),
                RefAsNonNull,
                LocalSet(stack_struct_local),
                LocalGet((func.params().len() - 2).try_into().map_err(|_| make_hq_bug!("local index out of bounds"))?),
                LocalGet(stack_struct_local),
                StructGet {
                    struct_type_index: stack_struct_ty,
                    field_index: 1,
                },
                LocalGet(stack_struct_local),
                StructGet {
                    struct_type_index: stack_struct_ty,
                    field_index: 0,
                },
                ReturnCallRef(step_func_ty),
                End,
            ]
        }
        YieldMode::Return => wasm![Return],
        YieldMode::Inline(step) => {
            hq_assert!(
                !RefCell::borrow(step).used_non_inline(),
                "inlined step should not be marked as used non-inline"
            );
            func.compile_inner_step(Rc::clone(step))?
        }
        YieldMode::Schedule(step_index) => {
            let threads_table = func
                .registries()
                .tables()
                .threads_table(func.target(), func.registries().types())?;
            let thread_struct_ty = func.registries().types().thread_struct_type()?;
            let local = func.local(ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::Concrete(thread_struct_ty),
            }))?;
            func.free_local(local)?;
            let stack_array_ty = func.registries().types().stack_array_type()?;
            let stack_struct_ty = func.registries().types().stack_struct_type()?;

            wasm![
                LocalGet(0),
                TableGet(threads_table),
                RefAsNonNull,
                LocalTee(local),
                StructGet { struct_type_index: thread_struct_ty, field_index: 1 },
                LocalGet(local),
                StructGet { struct_type_index: thread_struct_ty, field_index: 0 },
                I32Const(1),
                I32Sub,
                ArrayGet(stack_array_ty),
                RefAsNonNull,
                #LazyStepRef(*step_index),
                StructSet { struct_type_index: stack_struct_ty, field_index: 0 },
                Return
            ]
        }
    })
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
    use crate::instructions::tests::assert_valid_json;

    #[test]
    fn fields_display_is_valid_json() {
        let target = make_target();
        for mode in [
            YieldMode::None,
            YieldMode::Return,
            make_schedule(),
            YieldMode::Inline(Rc::new(RefCell::new(Step::new_empty(
                Weak::new(),
                false,
                target,
            )))),
        ] {
            assert_valid_json(format!("{}", Fields { mode }));
        }
    }

    pub fn make_schedule() -> YieldMode {
        YieldMode::Schedule(StepIndex(0))
    }

    pub fn make_inline() -> YieldMode {
        let target = make_target();
        YieldMode::Inline(Rc::new(RefCell::new(Step::new_empty(
            Weak::new(),
            false,
            target,
        ))))
    }
}

instructions_test! (
    mod test_none for hq_yield {
        fields = super::Fields { mode: super::YieldMode::None };
    }
);

instructions_test! (
    mod test_return for hq_yield {
        fields = super::Fields { mode: super::YieldMode::Return };
    }
);

instructions_test! (
    mod test_schedule for hq_yield {
        fields = super::Fields {
            mode: super::test::make_schedule()
        };
    }
);

instructions_test!(
    mod test_inline for hq_yield {
        fields = super::Fields {
            mode: super::test::make_inline()
        };
    }
);
