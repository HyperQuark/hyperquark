use wasm_encoder::BlockType;

use super::super::prelude::*;
use crate::instructions_test;
use crate::ir::{Step, StepIndex};
use crate::wasm::StepFunc;
use crate::wasm::registries::functions::static_functions::{DynArrayGet, DynArrayLen, DynArrayPop};
use crate::wasm::registries::types::{
    TDynArray, TNonNullable, TNullable, TStackStruct, TStepFunc, TType,
};

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
    Ok(match yield_mode {
        YieldMode::None => {
            let static_functions = Rc::clone(func.registries().static_functions());
            let types = Rc::clone(func.registries().types());

            let pop_stack =
                static_functions.register::<DynArrayPop<TNullable<TStackStruct>>, _>()?;
            let stack_len =
                static_functions.register::<DynArrayLen<TNullable<TStackStruct>>, _>()?;
            let stack_get =
                static_functions.register::<DynArrayGet<TNullable<TStackStruct>>, _>()?;

            let stack_array_ty = <TDynArray<TNullable<TStackStruct>>>::ty(&types)?;
            let step_struct_ty = TStackStruct::ty(&types)?;

            let stack_local = (func.params().len() - 2) as u32;

            let stack_len_local = func.local(ValType::I32)?;
            let step_struct_local = func.local(<TNonNullable<TStackStruct>>::ty(&types)?)?;
            func.free_local(step_struct_local)?;
            func.free_local(stack_len_local)?;

            wasm![
                LocalGet(stack_local),
                RefCastNonNull(stack_array_ty),
                #StaticFunctionCall(pop_stack),
                Drop,
                LocalGet(stack_local),
                RefCastNonNull(stack_array_ty),
                #StaticFunctionCall(stack_len),
                LocalTee(stack_len_local),
                I32Eqz,
                If(BlockType::Empty),
                // Empty stack cleanup (if it happens at all) will happen in scheduler, not here.
                Return,
                Else,
                LocalGet(stack_local),
                LocalGet(stack_local),
                LocalGet(stack_local),
                RefCastNonNull(stack_array_ty),
                LocalGet(stack_len_local),
                I32Const(1),
                I32Sub,
                #StaticFunctionCall(stack_get),
                RefAsNonNull,
                LocalTee(step_struct_local),
                StructGet {
                    struct_type_index: step_struct_ty,
                    field_index: 1,
                },
                LocalGet(step_struct_local),
                StructGet {
                    struct_type_index: step_struct_ty,
                    field_index: 0,
                },
                ReturnCallRef(TStepFunc::ty(&types)?),
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
            let static_functions = Rc::clone(func.registries().static_functions());
            let types = Rc::clone(func.registries().types());

            let stack_array_ty = <TDynArray<TNullable<TStackStruct>>>::ty(&types)?;
            let step_func_ty = TStepFunc::ty(&types)?;

            let stack_local = (func.params().len() - 2) as u32;

            let stack_len =
                static_functions.register::<DynArrayLen<TNullable<TStackStruct>>, _>()?;
            let stack_get =
                static_functions.register::<DynArrayGet<TNullable<TStackStruct>>, _>()?;

            wasm![
                LocalGet(stack_local),
                RefCastNonNull(stack_array_ty),
                LocalGet(stack_local),
                RefCastNonNull(stack_array_ty),
                #StaticFunctionCall(stack_len),
                I32Const(1),
                I32Sub,
                #StaticFunctionCall(stack_get),
                RefAsNonNull,
                #LazyStepRef(*step_index),
                RefCastNonNull(step_func_ty),
                StructSet { struct_type_index: TStackStruct::ty(&types)?, field_index: 0 },
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
