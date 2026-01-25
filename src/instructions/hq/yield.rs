use wasm_encoder::{BlockType, ConstExpr, HeapType};

use super::super::prelude::*;
use crate::ir::{Step, StepIndex};
use crate::wasm::{GlobalExportable, GlobalMutable, StepFunc, ThreadsTable};

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
    let threads_count = func.registries().globals().register(
        "threads_count".into(),
        (
            ValType::I32,
            ConstExpr::i32_const(0),
            GlobalMutable(true),
            GlobalExportable(true),
        ),
    )?;

    Ok(match yield_mode {
        YieldMode::None => {
            let threads_table = func.registries().tables().register::<ThreadsTable, _>()?;
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
            let step_func_ty = func.registries().types().step_func_type()?;

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
            let threads_table = func.registries().tables().register::<ThreadsTable, _>()?;
            let thread_struct_ty = func.registries().types().thread_struct_type()?;
            let local = func.local(ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::Concrete(thread_struct_ty),
            }))?;
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

crate::instructions_test! {none; hq_yield; @ super::Fields { mode: super::YieldMode::None }}

crate::instructions_test! {ret; hq_yield; @ super::Fields { mode: super::YieldMode::Return }}

// crate::instructions_test! {
//     schedule;
//     hq_yield;
//     @ super::Fields {
//         mode: super::YieldMode::Schedule(
//             crate::rc::Rc::downgrade(&crate::ir::Step::new_empty(
//                 &crate::rc::Rc::downgrade(&Rc::new(crate::ir::IrProject::new(BTreeMap::default(), BTreeMap::default(), Box::from([])))),
//                 true,
//                 Rc::new(
//                     crate::ir::Target::new(
//                         false,
//                         BTreeMap::default(),
//                         BTreeMap::default(),
//                         crate::rc::Rc::downgrade(&Rc::new(crate::ir::IrProject::new(BTreeMap::default(), BTreeMap::default(), Box::from([])))),
//                         RefCell::default(),
//                         0,
//                         Box::from([])
//                     )
//                 )
//             ).unwrap())
//         )
//     }
// }
