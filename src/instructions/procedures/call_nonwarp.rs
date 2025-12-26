use super::super::prelude::*;
use crate::ir::{Proc, Step};
use crate::wasm::{StepFunc, ThreadsTable, WasmProject};
use wasm_encoder::{FieldType, HeapType, Instruction as WInstruction, StorageType};

#[derive(Clone, Debug)]
pub struct Fields {
    pub proc: Rc<Proc>,
    pub next_step: Rc<Step>,
}

impl fmt::Display for Fields {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            r#"{{
        "proc": {:?},
    }}"#,
            self.proc.proccode()
        )
    }
}

pub fn wasm(
    func: &StepFunc,
    inputs: Rc<[IrType]>,
    Fields { proc, next_step }: &Fields,
) -> HQResult<Vec<InternalInstruction>> {
    let Some(ref nonwarped_specific_proc) = *proc.nonwarped_specific_proc() else {
        hq_bug!("nonwarped_specific_proc didn't exist for call_nonwarp")
    };

    let arg_struct_type = func.registries().types().struct_(
        (*nonwarped_specific_proc.arg_vars())
            .borrow()
            .iter()
            .map(|var| {
                Ok(FieldType {
                    mutable: false,
                    element_type: StorageType::Val(WasmProject::ir_type_to_wasm(
                        *var.possible_types(),
                    )?),
                })
            })
            .collect::<HQResult<Vec<_>>>()?,
    )?;
    let arg_struct_local = func.local(ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(arg_struct_type),
    }))?;
    let stack_struct_type = func.registries().types().stack_struct_type()?;
    let stack_struct_local = func.local(ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(stack_struct_type),
    }))?;
    let stack_array_type = func.registries().types().stack_array_type()?;
    let thread_struct_type = func.registries().types().thread_struct_type()?;
    let thread_struct_local = func.local(ValType::Ref(RefType {
        nullable: false,
        heap_type: HeapType::Concrete(thread_struct_type),
    }))?;
    let threads_table = func.registries().tables().register::<ThreadsTable, _>()?;

    let locals = inputs
        .iter()
        .map(|ty| func.local(WasmProject::ir_type_to_wasm(*ty)?))
        .collect::<HQResult<Vec<_>>>()?;

    let mut wasm = locals
        .iter()
        .rev()
        .copied()
        .map(WInstruction::LocalSet)
        .map(InternalInstruction::Immediate)
        .collect::<Vec<_>>();

    for ((&input, local), param) in inputs.iter().zip(locals).zip(
        nonwarped_specific_proc
            .arg_vars()
            .try_borrow()?
            .iter()
            .map(|var| **var.possible_types().borrow()),
    ) {
        wasm.extend(if param.is_base_type() {
            wasm![LocalGet(local)]
        } else {
            wasm![
                LocalGet(local),
                @boxed(input),
            ]
        });
    }

    wasm.extend(wasm![
        StructNew(arg_struct_type),
        LocalSet(arg_struct_local),
        #LazyNonWarpedProcRef(Rc::clone(proc)),
        LocalGet(arg_struct_local),
        StructNew(stack_struct_type),
        LocalSet(stack_struct_local),
        LocalGet((func.params().len() - 2).try_into().map_err(|_| make_hq_bug!("local index out of bounds"))?),
        TableGet(threads_table),
        RefAsNonNull,
        LocalTee(thread_struct_local),
        StructGet {
            struct_type_index: thread_struct_type,
            field_index: 1,
        },
        LocalGet(thread_struct_local),
        StructGet {
            struct_type_index: thread_struct_type,
            field_index: 0,
        },
        LocalGet(stack_struct_local),
        // todo: consider the case where we need to resize the array
        ArraySet(stack_array_type),
        LocalGet(thread_struct_local),
        StructGet {
            struct_type_index: thread_struct_type,
            field_index: 1,
        },
        LocalGet(thread_struct_local),
        StructGet {
            struct_type_index: thread_struct_type,
            field_index: 0,
        },
        I32Const(1),
        I32Sub,
        ArrayGet(stack_array_type),
        #LazyStepRef(Rc::downgrade(next_step)),
        StructSet {
            struct_type_index: stack_struct_type,
            field_index: 0,
        },

        LocalGet(thread_struct_local),
        LocalGet(thread_struct_local),
        StructGet {
            struct_type_index: thread_struct_type,
            field_index: 0,
        },
        I32Const(1),
        I32Add,
        StructSet {
            struct_type_index: thread_struct_type,
            field_index: 0,
        },
        LocalGet((func.params().len() - 2).try_into().map_err(|_| make_hq_bug!("local index out of bounds"))?),
        LocalGet(arg_struct_local),
        #LazyNonWarpedProcRef(Rc::clone(proc)),
        CallRef(func.registries().types().step_func_type()?)
    ]);

    Ok(wasm)
}

pub fn acceptable_inputs(Fields { proc, .. }: &Fields) -> HQResult<Rc<[IrType]>> {
    let Some(ref nonwarped_specific_proc) = *proc.nonwarped_specific_proc() else {
        hq_bug!("nonwarped_specific_proc didn't exist for call_nonwarp")
    };

    Ok(nonwarped_specific_proc
        .arg_vars()
        .try_borrow()?
        .iter()
        .map(|var| *var.possible_types())
        .collect())
}

pub fn output_type(_inputs: Rc<[IrType]>, _fields: &Fields) -> HQResult<ReturnType> {
    Ok(ReturnType::None)
}

pub const REQUESTS_SCREEN_REFRESH: bool = false;
