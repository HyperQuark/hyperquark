use super::super::prelude::*;
use crate::ir::Step;
use crate::wasm::TableOptions;
use crate::wasm::{flags::Scheduler, GlobalExportable, GlobalMutable, StepFunc};
use wasm_encoder::{ConstExpr, HeapType, MemArg};

#[derive(Clone, Debug)]
pub enum YieldMode {
    Inline(Rc<Step>),
    Schedule(Weak<Step>),
    None,
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
            }
        )?;
        match self {
            Self::Inline(step) => {
                write!(f, r#", "step": {:?}"#, step.id())?;
            }
            Self::Schedule(step) => {
                write!(
                    f,
                    r#", "step": {:?}"#,
                    match step.upgrade() {
                        Some(ref rcstep) => rcstep.id(),
                        None => return Err(fmt::Error),
                    }
                )?;
            }
            Self::None => (),
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
    let noop_global = func.registries().globals().register(
        "noop_func".into(),
        (
            ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::Concrete(
                    func.registries()
                        .types()
                        .register_default((vec![ValType::I32], vec![]))?,
                ),
            }),
            ConstExpr::ref_func(0), // this is a placeholder.
            GlobalMutable(false),
            GlobalExportable(false),
        ),
    )?;

    let step_func_ty = func
        .registries()
        .types()
        .register_default((vec![ValType::I32], vec![]))?;

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
        YieldMode::None => match func.flags().scheduler {
            Scheduler::CallIndirect => {
                // Write a special value (e.g. 0 for noop) to linear memory for this thread
                wasm![
                    LocalGet(0), // thread index
                    I32Const(4),
                    I32Mul,
                    I32Const(0), // 0 = noop step index
                    // store at address (thread_index * 4)
                    I32Store(MemArg {
                        offset: 0,
                        align: 2,
                        memory_index: 0,
                    }),
                    // GlobalGet(threads_count),
                    // I32Const(1),
                    // I32Sub,
                    // GlobalSet(threads_count),
                    Return
                ]
            }
            Scheduler::TypedFuncRef => {
                let threads_table = func.registries().tables().register(
                    "threads".into(),
                    TableOptions {
                        element_type: RefType {
                            nullable: false,
                            heap_type: HeapType::Concrete(step_func_ty),
                        },
                        min: 0,
                        max: None,
                        // this default gets fixed up in src/wasm/tables.rs
                        init: None,
                    },
                )?;
                wasm![
                    LocalGet(0),
                    #LazyGlobalGet(noop_global),
                    TableSet(threads_table),
                    #LazyGlobalGet(threads_count),
                    I32Const(1),
                    I32Sub,
                    #LazyGlobalSet(threads_count),
                    Return
                ]
            }
        },
        YieldMode::Inline(step) => {
            hq_assert!(
                !step.used_non_inline(),
                "inlined step should not be marked as used non-inline"
            );
            func.compile_inner_step(step)?
        }
        YieldMode::Schedule(weak_step) => {
            let step = Weak::upgrade(weak_step)
                .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Step>"))?;
            hq_assert!(
                step.used_non_inline(),
                "scheduled step should be marked as used non-inline"
            );
            match func.flags().scheduler {
                Scheduler::CallIndirect => {
                    wasm![
                        LocalGet(0), // thread index
                        I32Const(4),
                        I32Mul,
                        #LazyStepIndex(Weak::clone(weak_step)),
                        I32Store(MemArg {
                            offset: 0,
                            align: 2,
                            memory_index: 0,
                        }),
                        Return
                    ]
                }
                Scheduler::TypedFuncRef => {
                    let threads_table = func.registries().tables().register(
                        "threads".into(),
                        TableOptions {
                            element_type: RefType {
                                nullable: false,
                                heap_type: HeapType::Concrete(step_func_ty),
                            },
                            min: 0,
                            max: None,
                            // this default gets fixed up in src/wasm/tables.rs
                            init: None,
                        },
                    )?;
                    wasm![
                        LocalGet(0),
                        #LazyStepRef(Weak::clone(weak_step)),
                        TableSet(threads_table),
                        Return
                    ]
                }
            }
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

crate::instructions_test! {none; hq_yield; @ super::Fields { mode: super::YieldMode::None }}
