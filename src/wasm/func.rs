use super::{Registries, WasmFlags, WasmProject};
use crate::instructions::IrOpcode;
use crate::ir::{PartialStep, RcVar, ReturnType, Step};
use crate::prelude::*;
use crate::{instructions::wrap_instruction, ir::Proc};
use alloc::collections::btree_map;
use wasm_encoder::{
    self, CodeSection, Function, FunctionSection, Instruction as WInstruction, ValType,
};

#[derive(Clone, Debug)]
pub enum Instruction {
    Immediate(wasm_encoder::Instruction<'static>),
    LazyStepRef(Weak<Step>),
    LazyStepIndex(Weak<Step>),
    LazyWarpedProcCall(Rc<Proc>),
    LazyGlobalGet(u32),
    LazyGlobalSet(u32),
}

impl Instruction {
    pub fn eval(
        &self,
        steps: &Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>>,
        imported_func_count: u32,
        imported_global_count: u32,
    ) -> HQResult<WInstruction<'static>> {
        Ok(match self {
            Self::Immediate(instr) => instr.clone(),
            Self::LazyStepRef(step) => {
                let step_index: u32 = steps
                    .try_borrow()?
                    .get_index_of(
                        &step
                            .upgrade()
                            .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Step>"))?,
                    )
                    .ok_or_else(|| make_hq_bug!("couldn't find step in step map"))?
                    .try_into()
                    .map_err(|_| make_hq_bug!("step index out of bounds"))?;
                WInstruction::RefFunc(imported_func_count + step_index)
            }
            Self::LazyStepIndex(step) => {
                let step_index: i32 = steps
                    .try_borrow()?
                    .get_index_of(
                        &step
                            .upgrade()
                            .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak<Step>"))?,
                    )
                    .ok_or_else(|| make_hq_bug!("couldn't find step in step map"))?
                    .try_into()
                    .map_err(|_| make_hq_bug!("step index out of bounds"))?;
                WInstruction::I32Const(step_index)
            }
            Self::LazyWarpedProcCall(proc) => {
                hq_assert!(
                    proc.context().always_warped(),
                    "tried to use LazyWarpedProcCall on a non-warped step"
                );
                let PartialStep::Finished(ref step) = *proc.first_step()? else {
                    hq_bug!("tried to use uncompiled procedure step")
                };
                let step_index: u32 = steps
                    .try_borrow()?
                    .get_index_of(step)
                    .ok_or_else(|| make_hq_bug!("couldn't find step in step map"))?
                    .try_into()
                    .map_err(|_| make_hq_bug!("step index out of bounds"))?;
                WInstruction::Call(imported_func_count + step_index)
            }
            Self::LazyGlobalGet(idx) => {
                crate::log!("global get {idx}. imported globals: {imported_global_count}");
                WInstruction::GlobalGet(idx + imported_global_count)
            }
            Self::LazyGlobalSet(idx) => {
                crate::log!("global get {idx}. imported globals: {imported_global_count}");
                WInstruction::GlobalSet(idx + imported_global_count)
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
    /// is obtained from the SpriteRegistry, as the stage is not a sprite but holds a target index
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
}

impl StepFunc {
    pub fn registries(&self) -> Rc<Registries> {
        Rc::clone(&self.registries)
    }

    pub const fn flags(&self) -> WasmFlags {
        self.flags
    }

    pub const fn instructions(&self) -> &RefCell<Vec<Instruction>> {
        &self.instructions
    }

    pub fn params(&self) -> &[ValType] {
        &self.params
    }

    pub fn target(&self) -> StepTarget {
        self.target
    }

    pub fn target_index(&self) -> u32 {
        self.target_index
    }

    /// creates a new step function, with one paramter
    pub fn new(
        registries: Rc<Registries>,
        flags: WasmFlags,
        target: StepTarget,
        target_index: u32,
    ) -> Self {
        Self {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            params: Box::new([ValType::I32]),
            output: Box::new([]),
            registries,
            flags,
            local_variables: RefCell::new(BTreeMap::default()),
            target,
            target_index,
        }
    }

    /// creates a new step function with the specified amount of paramters.
    /// currently only used in testing to validate types
    pub fn new_with_types(
        params: Box<[ValType]>,
        output: Box<[ValType]>,
        registries: Rc<Registries>,
        flags: WasmFlags,
        target: StepTarget,
        target_index: u32,
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
        steps: &Rc<RefCell<IndexMap<Rc<Step>, Self>>>,
        imported_func_count: u32,
        imported_global_count: u32,
    ) -> HQResult<()> {
        let mut func = Function::new_with_locals_types(self.locals.take());
        for instruction in self.instructions().take() {
            func.instruction(&instruction.eval(
                steps,
                imported_func_count,
                imported_global_count,
            )?);
        }
        func.instruction(&wasm_encoder::Instruction::End);
        let type_index = self
            .registries()
            .types()
            .register_default((self.params.into(), self.output.into()))?;
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
                ReturnType::MultiValue(outputs) => type_stack.extend(outputs.into_iter().copied()),
                ReturnType::None => (),
            };
        }
        Ok(instrs)
    }

    pub fn compile_step(
        step: Rc<Step>,
        steps: &Rc<RefCell<IndexMap<Rc<Step>, Self>>>,
        registries: Rc<Registries>,
        flags: WasmFlags,
    ) -> HQResult<Self> {
        hq_assert!(
            step.used_non_inline(),
            "step should be marked as used non-inline to compile normally"
        );
        if let Some(step_func) = steps.try_borrow()?.get(&step) {
            return Ok(step_func.clone());
        }
        let target = if step.context().target().is_stage() {
            StepTarget::Stage
        } else {
            StepTarget::Sprite(
                registries
                    .sprites()
                    .register_default(Rc::clone(step.context().target()))?,
            )
        };
        let target_index = step.context().target().index();
        let step_func = if let Some(ref proc_context) = step.context().proc_context {
            let arg_types = proc_context
                .arg_vars()
                .try_borrow()?
                .iter()
                .map(|var| WasmProject::ir_type_to_wasm(*var.possible_types()))
                .collect::<HQResult<Box<[_]>>>()?;
            let params = arg_types.iter().chain(&[ValType::I32]).copied().collect();
            let outputs = proc_context
                .return_vars()
                .try_borrow()?
                .iter()
                .map(|var| WasmProject::ir_type_to_wasm(*var.possible_types()))
                .collect::<HQResult<Box<[_]>>>()?;
            Self::new_with_types(params, outputs, registries, flags, target, target_index)
        } else {
            Self::new(registries, flags, target, target_index)
        };
        let instrs = Self::compile_instructions(&step_func, &*step.opcodes().try_borrow()?)?;
        step_func.add_instructions(instrs)?;
        steps
            .try_borrow_mut()
            .map_err(|_| make_hq_bug!("couldn't mutably borrow cell"))?
            .insert(step, step_func.clone());
        Ok(step_func)
    }

    pub fn compile_inner_step(&self, step: &Rc<Step>) -> HQResult<Vec<Instruction>> {
        hq_assert!(
            !step.used_non_inline(),
            "inner step should NOT be marked as used non-inline"
        );
        Self::compile_instructions(self, &*step.opcodes().try_borrow()?)
    }
}
