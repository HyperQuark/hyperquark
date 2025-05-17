use super::{Registries, WasmFlags, WasmProject};
use crate::ir::{PartialStep, RcVar, Step};
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
}

impl Instruction {
    pub fn eval(
        &self,
        steps: &Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>>,
        imported_func_count: u32,
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
                let PartialStep::Finished(ref step) = *proc.warped_first_step()? else {
                    hq_bug!("tried to use uncompiled warped procedure step")
                };
                let step_index: u32 = steps
                    .try_borrow()?
                    .get_index_of(step)
                    .ok_or_else(|| make_hq_bug!("couldn't find step in step map"))?
                    .try_into()
                    .map_err(|_| make_hq_bug!("step index out of bounds"))?;
                WInstruction::Call(imported_func_count + step_index)
            }
        })
    }
}

/// representation of a step's function
#[derive(Clone)]
pub struct StepFunc {
    locals: RefCell<Vec<ValType>>,
    instructions: RefCell<Vec<Instruction>>,
    params: Box<[ValType]>,
    output: Option<ValType>,
    registries: Rc<Registries>,
    flags: WasmFlags,
    steps: Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>>,
    local_variables: RefCell<BTreeMap<RcVar, u32>>,
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

    pub fn steps(&self) -> Rc<RefCell<IndexMap<Rc<Step>, Self>>> {
        Rc::clone(&self.steps)
    }

    pub fn params(&self) -> &[ValType] {
        &self.params
    }

    /// creates a new step function, with one paramter
    pub fn new(
        registries: Rc<Registries>,
        steps: Rc<RefCell<IndexMap<Rc<Step>, Self>>>,
        flags: WasmFlags,
    ) -> Self {
        Self {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            params: Box::new([ValType::I32]),
            output: None,
            registries,
            flags,
            steps,
            local_variables: RefCell::new(BTreeMap::default()),
        }
    }

    /// creates a new step function with the specified amount of paramters.
    /// currently only used in testing to validate types
    pub fn new_with_types(
        params: Box<[ValType]>,
        output: Option<ValType>,
        registries: Rc<Registries>,
        steps: Rc<RefCell<IndexMap<Rc<Step>, Self>>>,
        flags: WasmFlags,
    ) -> Self {
        Self {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            params,
            output,
            registries,
            flags,
            steps,
            local_variables: RefCell::new(BTreeMap::default()),
        }
    }

    pub fn local_variable(&self, var: &RcVar) -> HQResult<u32> {
        crate::log!("accessing local variable for variable {}", var.id());
        crate::log!(
            "existing local variables: {:?}",
            self.local_variables.borrow()
        );
        Ok(
            match self.local_variables.try_borrow_mut()?.entry(var.clone()) {
                btree_map::Entry::Occupied(entry) => {
                    crate::log("local already exists, returning that");
                    *entry.get()
                }
                btree_map::Entry::Vacant(entry) => {
                    crate::log("making a new local for variable");
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
        self.instructions
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
    ) -> HQResult<()> {
        let mut func = Function::new_with_locals_types(self.locals.take());
        for instruction in self.instructions.take() {
            func.instruction(&instruction.eval(steps, imported_func_count)?);
        }
        func.instruction(&wasm_encoder::Instruction::End);
        let type_index = self.registries().types().register_default((
            self.params.into(),
            self.output.map_or_else(Vec::new, |output| vec![output]),
        ))?;
        funcs.function(type_index);
        code.function(&func);
        Ok(())
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
        let step_func = if let Some(ref proc_context) = step.context().proc_context {
            let arg_types = proc_context
                .arg_types()
                .iter()
                .copied()
                .map(WasmProject::ir_type_to_wasm)
                .collect::<HQResult<Box<[_]>>>()?;
            let input_types = arg_types.iter().chain(&[ValType::I32]).copied().collect();
            Self::new_with_types(input_types, None, registries, Rc::clone(steps), flags)
        } else {
            Self::new(registries, Rc::clone(steps), flags)
        };
        let mut instrs = vec![];
        let mut type_stack = vec![];
        for opcode in &*step.opcodes().try_borrow()?.clone() {
            let inputs = type_stack
                .splice((type_stack.len() - opcode.acceptable_inputs()?.len()).., [])
                .collect();
            instrs.append(&mut wrap_instruction(
                &step_func,
                Rc::clone(&inputs),
                opcode,
            )?);
            if let Some(output) = opcode.output_type(inputs)? {
                type_stack.push(output);
            }
        }
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
        let mut instrs = vec![];
        let mut type_stack = vec![];
        for opcode in &*step.opcodes().try_borrow()?.clone() {
            let inputs = type_stack
                .splice((type_stack.len() - opcode.acceptable_inputs()?.len()).., [])
                .collect();
            instrs.append(&mut wrap_instruction(self, Rc::clone(&inputs), opcode)?);
            if let Some(output) = opcode.output_type(inputs)? {
                type_stack.push(output);
            }
        }
        Ok(instrs)
    }
}
