use alloc::collections::btree_map;
use core::ops::Deref;

use wasm_encoder::{
    self, AbstractHeapType, CodeSection, FieldType, Function, FunctionSection, HeapType,
    Instruction as WInstruction, RefType, StorageType, ValType,
};
use wasm_gen::wasm;

use super::{Registries, WasmFlags, WasmProject};
use crate::instructions::{IrOpcode, wrap_instruction};
use crate::ir::{Event, PartialStep, Proc, RcVar, ReturnType, Step, StepIndex};
use crate::prelude::*;
use crate::wasm::registries::TypeRegistry;

#[derive(Clone, Debug)]
pub enum Instruction {
    Immediate(wasm_encoder::Instruction<'static>),
    LazyStepRef(StepIndex),
    LazyWarpedProcCall(Rc<Proc>),
    LazyNonWarpedProcRef(Rc<Proc>),
    LazyGlobalGet(u32),
    LazyGlobalSet(u32),
    LazyBroadcastSpawn(Box<str>),
    LazyBroadcastSpawnAndWait((Box<str>, StepIndex, StepIndex, u32)),
    StaticFunctionCall(u32),
}

impl Instruction {
    pub fn eval(
        &self,
        events: &BTreeMap<Event, Vec<u32>>,
        types: &Rc<TypeRegistry>,
        threads_count_global: u32,
        spawn_new_thread_func: u32,
        spawn_thread_in_stack_func: u32,
        threads_table: u32,
        imported_func_count: u32,
        static_func_count: u32,
        imported_global_count: u32,
    ) -> HQResult<Box<[WInstruction<'static>]>> {
        Ok(match self {
            Self::Immediate(instr) => Box::from([instr.clone()]),
            #[cfg(test)]
            Self::LazyStepRef(_step) => Box::from([WInstruction::RefFunc(
                imported_func_count + static_func_count,
            )]),
            #[cfg(not(test))]
            Self::LazyStepRef(step_index) => Box::from([WInstruction::RefFunc(
                imported_func_count
                    + static_func_count
                    + u32::try_from(step_index.0)
                        .map_err(|_| make_hq_bug!("step index out of bounds"))?,
            )]),
            Self::LazyBroadcastSpawn(broadcast) => {
                let broadcast_indices = events
                    .get(&Event::Broadcast(broadcast.clone()))
                    .cloned()
                    .unwrap_or_default();

                // todo: these should begin execution in the same step, I think, possibly immediately?

                broadcast_indices
                    .iter()
                    .flat_map(|&i| {
                        [
                            WInstruction::RefFunc(i + imported_func_count + static_func_count),
                            WInstruction::RefNull(HeapType::Abstract {
                                shared: false,
                                ty: AbstractHeapType::Struct,
                            }),
                            WInstruction::Call(spawn_new_thread_func + imported_func_count),
                        ]
                    })
                    .chain([
                        WInstruction::GlobalGet(threads_count_global + imported_global_count),
                        WInstruction::I32Const(
                            i32::try_from(broadcast_indices.len())
                                .map_err(|_| make_hq_bug!("indices len out of bounds"))?,
                        ),
                        WInstruction::I32Add,
                        WInstruction::GlobalSet(threads_count_global + imported_global_count),
                    ])
                    .collect()
            }
            Self::LazyBroadcastSpawnAndWait((broadcast, poll_step, next_step, arr_local)) => {
                let broadcast_indices = events
                    .get(&Event::Broadcast(broadcast.clone()))
                    .cloned()
                    .unwrap_or_default();

                let i32_array_type = types.array(StorageType::Val(ValType::I32), true)?;
                let thread_poll_struct = types.struct_(vec![FieldType {
                    element_type: StorageType::Val(ValType::Ref(RefType {
                        nullable: false,
                        heap_type: HeapType::Concrete(i32_array_type),
                    })),
                    mutable: false,
                }])?;

                let poll_step_index: u32 = poll_step
                    .0
                    .try_into()
                    .map_err(|_| make_hq_bug!("poll_step index out of bounds"))?;

                let next_step_index: u32 = next_step
                    .0
                    .try_into()
                    .map_err(|_| make_hq_bug!("next_step index out of bounds"))?;

                let broadcast_num = i32::try_from(broadcast_indices.len())
                    .map_err(|_| make_hq_bug!("indices len out of bounds"))?;

                [
                    WInstruction::I32Const(broadcast_num),
                    WInstruction::ArrayNewDefault(i32_array_type),
                    WInstruction::LocalSet(*arr_local),
                ]
                .into_iter()
                .chain(
                    broadcast_indices
                        .iter()
                        .enumerate()
                        .map(|(j, &i)| {
                            // todo: should these should begin execution in the same step?
                            Ok([
                                WInstruction::LocalGet(*arr_local),
                                WInstruction::I32Const(
                                    j.try_into()
                                        .map_err(|_| make_hq_bug!("index out of bounds"))?,
                                ),
                                WInstruction::TableSize(threads_table),
                                WInstruction::ArraySet(i32_array_type),
                                WInstruction::RefFunc(i + imported_func_count + static_func_count),
                                WInstruction::RefNull(HeapType::Abstract {
                                    shared: false,
                                    ty: AbstractHeapType::Struct,
                                }),
                                WInstruction::Call(spawn_new_thread_func + imported_func_count),
                            ])
                        })
                        .collect::<HQResult<Box<[_]>>>()?
                        .into_iter()
                        .flatten(),
                )
                .chain([
                    WInstruction::GlobalGet(threads_count_global + imported_global_count),
                    WInstruction::I32Const(broadcast_num),
                    WInstruction::I32Add,
                    WInstruction::GlobalSet(threads_count_global + imported_global_count),
                    WInstruction::RefFunc(
                        poll_step_index + imported_func_count + static_func_count,
                    ),
                    WInstruction::LocalGet(*arr_local),
                    WInstruction::StructNew(thread_poll_struct),
                    WInstruction::RefFunc(
                        next_step_index + imported_func_count + static_func_count,
                    ),
                    WInstruction::Call(spawn_thread_in_stack_func + imported_func_count),
                ])
                .collect()
            }
            Self::LazyWarpedProcCall(proc) => {
                let Some(ref warped_specific_proc) = *proc.warped_specific_proc() else {
                    hq_bug!("tried to use LazyWarpedProcCall on a non-warped step")
                };
                let PartialStep::Finished(step_index) = *warped_specific_proc.first_step()? else {
                    hq_bug!("tried to use uncompiled procedure step")
                };
                Box::from([WInstruction::Call(
                    imported_func_count
                        + static_func_count
                        + u32::try_from(step_index.0)
                            .map_err(|_| make_hq_bug!("step index out of bounds"))?,
                )])
            }
            Self::LazyNonWarpedProcRef(proc) => {
                let Some(ref nonwarped_specific_proc) = *proc.nonwarped_specific_proc() else {
                    hq_bug!("tried to use LazyNonWarpedProcRef on a non-non-warped step")
                };
                let PartialStep::Finished(step_index) = *nonwarped_specific_proc.first_step()?
                else {
                    hq_bug!("tried to use uncompiled procedure step")
                };
                Box::from([WInstruction::RefFunc(
                    imported_func_count
                        + static_func_count
                        + u32::try_from(step_index.0)
                            .map_err(|_| make_hq_bug!("step index out of bounds"))?,
                )])
            }
            Self::LazyGlobalGet(idx) => {
                // crate::log!("global get {idx}. imported globals: {imported_global_count}");
                Box::from([WInstruction::GlobalGet(idx + imported_global_count)])
            }
            Self::LazyGlobalSet(idx) => {
                // crate::log!("global get {idx}. imported globals: {imported_global_count}");
                Box::from([WInstruction::GlobalSet(idx + imported_global_count)])
            }
            Self::StaticFunctionCall(idx) => {
                Box::from([WInstruction::Call(imported_func_count + idx)])
            }
        })
    }
}

#[derive(Clone, Copy)]
pub enum StepTarget {
    Stage,
    /// Holds the WASM index of the sprite.
    ///
    /// This is not necessarily the same as the IR index, as it
    /// is obtained from the `SpriteRegistry`, as the stage is not a sprite but holds a target index
    /// in the IR.
    Sprite(u32),
}

/// representation of a step's function
#[derive(Clone)]
pub struct StepFunc {
    locals: RefCell<Vec<ValType>>,
    instructions: RefCell<Vec<Instruction>>,
    params: Box<[ValType]>,
    output: Box<[ValType]>,
    registries: Rc<Registries>,
    flags: WasmFlags,
    local_variables: RefCell<BTreeMap<RcVar, u32>>,
    target: StepTarget,
    // the actual target index, for interfacing with js
    target_index: u32,
    costume_names: Rc<Vec<Vec<Box<str>>>>,
}

impl StepFunc {
    pub fn registries(&self) -> Rc<Registries> {
        Rc::clone(&self.registries)
    }

    pub const fn flags(&self) -> &WasmFlags {
        &self.flags
    }

    pub const fn instructions(&self) -> &RefCell<Vec<Instruction>> {
        &self.instructions
    }

    pub fn params(&self) -> &[ValType] {
        &self.params
    }

    pub const fn target(&self) -> StepTarget {
        self.target
    }

    pub const fn target_index(&self) -> u32 {
        self.target_index
    }

    pub const fn costume_names(&self) -> &Rc<Vec<Vec<Box<str>>>> {
        &self.costume_names
    }

    /// creates a new step function, with one paramter
    #[must_use]
    pub fn new(
        registries: Rc<Registries>,
        flags: WasmFlags,
        target: StepTarget,
        target_index: u32,
        costume_names: Rc<Vec<Vec<Box<str>>>>,
    ) -> Self {
        Self {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            params: Box::new([ValType::I32, TypeRegistry::STRUCT_REF]),
            output: Box::new([]),
            registries,
            flags,
            local_variables: RefCell::new(BTreeMap::default()),
            target,
            target_index,
            costume_names,
        }
    }

    /// creates a new step function with the specified amount of paramters.
    /// currently only used in testing to validate types
    #[must_use]
    pub fn new_with_types(
        params: Box<[ValType]>,
        output: Box<[ValType]>,
        registries: Rc<Registries>,
        flags: WasmFlags,
        target: StepTarget,
        target_index: u32,
        costume_names: Rc<Vec<Vec<Box<str>>>>,
    ) -> Self {
        Self {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            params,
            output,
            registries,
            flags,
            local_variables: RefCell::new(BTreeMap::default()),
            target,
            target_index,
            costume_names,
        }
    }

    pub fn local_variable(&self, var: &RcVar) -> HQResult<u32> {
        // crate::log!("accessing local variable for variable {}", var.id());
        // crate::log!(
        //     "existing local variables: {:?}",
        //     self.local_variables.borrow()
        // );
        Ok(
            match self.local_variables.try_borrow_mut()?.entry(var.clone()) {
                btree_map::Entry::Occupied(entry) => {
                    // crate::log("local already exists, returning that");
                    *entry.get()
                }
                btree_map::Entry::Vacant(entry) => {
                    // crate::log("making a new local for variable");
                    let index = self.local(WasmProject::ir_type_to_wasm(*var.possible_types())?)?;
                    entry.insert(index);
                    index
                }
            },
        )
    }

    /// Registers a new local in this function, and returns its index
    pub fn local(&self, val_type: ValType) -> HQResult<u32> {
        self.locals
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .push(val_type);
        u32::try_from(self.locals.try_borrow()?.len() + self.params.len() - 1)
            .map_err(|_| make_hq_bug!("local index was out of bounds"))
    }

    pub fn add_instructions(
        &self,
        instructions: impl IntoIterator<Item = Instruction>,
    ) -> HQResult<()> {
        self.instructions()
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .extend(instructions);
        Ok(())
    }

    /// Takes ownership of the function and returns the backing `wasm_encoder` `Function`
    pub fn finish(
        self,
        funcs: &mut FunctionSection,
        code: &mut CodeSection,
        events: &BTreeMap<Event, Vec<u32>>,
        types: &Rc<TypeRegistry>,
        threads_count_global: u32,
        spawn_new_thread_func: u32,
        spawn_thread_in_stack_func: u32,
        threads_table: u32,
        imported_func_count: u32,
        static_func_count: u32,
        imported_global_count: u32,
    ) -> HQResult<()> {
        let mut func = Function::new_with_locals_types(self.locals.take());
        for instruction in self.instructions().take() {
            for real_instruction in instruction.eval(
                events,
                types,
                threads_count_global,
                spawn_new_thread_func,
                spawn_thread_in_stack_func,
                threads_table,
                imported_func_count,
                static_func_count,
                imported_global_count,
            )? {
                func.instruction(&real_instruction);
            }
        }
        func.instruction(&wasm_encoder::Instruction::End);
        let type_index = self
            .registries()
            .types()
            .function(self.params.into(), self.output.into())?;
        funcs.function(type_index);
        code.function(&func);
        Ok(())
    }

    fn compile_instructions(&self, opcodes: &Vec<IrOpcode>) -> HQResult<Vec<Instruction>> {
        let mut instrs = vec![];
        let mut type_stack = vec![];
        for opcode in opcodes {
            let inputs = type_stack
                .splice((type_stack.len() - opcode.acceptable_inputs()?.len()).., [])
                .collect();
            instrs.append(&mut wrap_instruction(self, Rc::clone(&inputs), opcode)?);
            match opcode.output_type(inputs)? {
                ReturnType::Singleton(output) => type_stack.push(output),
                ReturnType::MultiValue(outputs) => type_stack.extend(outputs.iter().copied()),
                ReturnType::None => (),
            }
        }
        Ok(instrs)
    }

    pub fn compile_step(
        step: &RefCell<Step>,
        step_index: StepIndex,
        steps: &Rc<RefCell<Vec<Self>>>,
        registries: Rc<Registries>,
        flags: WasmFlags,
        costume_names: Rc<Vec<Vec<Box<str>>>>,
    ) -> HQResult<Self> {
        hq_assert!(
            step.try_borrow()?.used_non_inline(),
            "step should be marked as used non-inline to compile normally"
        );
        hq_assert_eq!(
            step_index.0,
            steps.try_borrow()?.len(),
            "tried to compile step that wasn't the next one scheduled"
        );
        if let Some(step_func) = steps.try_borrow()?.get(step_index.0) {
            return Ok(step_func.clone());
        }
        let target = if step.try_borrow()?.context().target().is_stage() {
            StepTarget::Stage
        } else {
            StepTarget::Sprite(
                registries
                    .sprites()
                    .register_default(Rc::clone(step.try_borrow()?.context().target()))?,
            )
        };
        let target_index = step.try_borrow()?.context().target().index();
        let step_func = if let Some(ref proc_context) = step.try_borrow()?.context().proc_context {
            let params = if step.try_borrow()?.context().warp {
                let arg_types = (*proc_context.arg_vars)
                    .borrow()
                    .iter()
                    .map(|var| WasmProject::ir_type_to_wasm(*var.possible_types()))
                    .collect::<HQResult<Box<[_]>>>()?;
                arg_types
                    .iter()
                    .chain(&[ValType::I32, TypeRegistry::STRUCT_REF])
                    .copied()
                    .collect()
            } else {
                Box::from([ValType::I32, TypeRegistry::STRUCT_REF])
            };
            let outputs = if step.try_borrow()?.context().warp {
                (*proc_context.ret_vars)
                    .borrow()
                    .iter()
                    .map(|var| WasmProject::ir_type_to_wasm(*var.possible_types()))
                    .collect::<HQResult<Box<[_]>>>()?
            } else {
                Box::from([])
            };
            Self::new_with_types(
                params,
                outputs,
                registries,
                flags,
                target,
                target_index,
                costume_names,
            )
        } else {
            Self::new(registries, flags, target, target_index, costume_names)
        };
        if let Some(ref proc_context) = step.try_borrow()?.context().proc_context
            && !step.try_borrow()?.context().warp
        {
            let arg_struct_type = step_func
                .registries()
                .types()
                .proc_arg_struct_type(&(*proc_context.arg_vars).borrow())?;
            let struct_local = step_func.local(ValType::Ref(RefType {
                nullable: false,
                heap_type: HeapType::Concrete(arg_struct_type),
            }))?;
            hq_assert_eq!(struct_local, 2);
            step_func.add_instructions(wasm![
                LocalGet(1),
                RefCastNonNull(HeapType::Concrete(arg_struct_type)),
                LocalSet(2),
            ])?;
        }
        let instrs = Self::compile_instructions(&step_func, step.try_borrow()?.opcodes())?;
        step_func.add_instructions(instrs)?;
        steps
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .push(step_func.clone());
        Ok(step_func)
    }

    pub fn compile_inner_step<S>(&self, step: S) -> HQResult<Vec<Instruction>>
    where
        S: Deref<Target = RefCell<Step>>,
    {
        hq_assert!(
            !step.try_borrow()?.used_non_inline(),
            "inner step should NOT be marked as used non-inline"
        );
        Self::compile_instructions(self, step.try_borrow()?.opcodes())
    }
}
