use super::{Registries, WasmFlags};
use crate::instructions::wrap_instruction;
use crate::ir::Step;
use crate::prelude::*;
use wasm_encoder::{CodeSection, Function, FunctionSection, Instruction, ValType};

/// representation of a step's function
#[derive(Clone)]
pub struct StepFunc {
    locals: RefCell<Vec<ValType>>,
    instructions: RefCell<Vec<Instruction<'static>>>,
    params: Box<[ValType]>,
    output: Option<ValType>,
    registries: Rc<Registries>,
    flags: WasmFlags,
    steps: Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>>,
}

impl StepFunc {
    pub fn registries(&self) -> Rc<Registries> {
        Rc::clone(&self.registries)
    }

    pub fn flags(&self) -> WasmFlags {
        self.flags
    }

    pub fn instructions(&self) -> &RefCell<Vec<Instruction<'static>>> {
        &self.instructions
    }

    pub fn steps(&self) -> Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>> {
        Rc::clone(&self.steps)
    }

    /// creates a new step function, with one paramter
    pub fn new(
        registries: Rc<Registries>,
        steps: Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>>,
        flags: WasmFlags,
    ) -> Self {
        StepFunc {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            params: Box::new([ValType::I32]),
            output: Some(ValType::I32),
            registries,
            flags,
            steps,
        }
    }

    /// creates a new step function with the specified amount of paramters.
    /// currently only used in testing to validate types
    pub fn new_with_types(
        params: Box<[ValType]>,
        output: Option<ValType>,
        registries: Rc<Registries>,
        steps: Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>>,
        flags: WasmFlags,
    ) -> HQResult<Self> {
        Ok(StepFunc {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            params,
            output,
            registries,
            flags,
            steps,
        })
    }

    /// Registers a new local in this function, and returns its index
    pub fn local(&self, val_type: ValType) -> HQResult<u32> {
        self.locals.try_borrow_mut()?.push(val_type);
        u32::try_from(self.locals.try_borrow()?.len() + self.params.len() - 1)
            .map_err(|_| make_hq_bug!("local index was out of bounds"))
    }

    pub fn add_instructions(
        &self,
        instructions: impl IntoIterator<Item = Instruction<'static>>,
    ) -> HQResult<()> {
        self.instructions.try_borrow_mut()?.extend(instructions);
        Ok(())
    }

    /// Takes ownership of the function and returns the backing `wasm_encoder` `Function`
    pub fn finish(self, funcs: &mut FunctionSection, code: &mut CodeSection) -> HQResult<()> {
        let mut func = Function::new_with_locals_types(self.locals.take());
        for instruction in self.instructions.take() {
            func.instruction(&instruction);
        }
        func.instruction(&Instruction::End);
        let type_index = self.registries().types().register_default((
            self.params.into(),
            if let Some(output) = self.output {
                vec![output]
            } else {
                vec![]
            },
        ))?;
        funcs.function(type_index);
        code.function(&func);
        Ok(())
    }

    pub fn compile_step(
        step: Rc<Step>,
        steps: Rc<RefCell<IndexMap<Rc<Step>, StepFunc>>>,
        registries: Rc<Registries>,
        flags: WasmFlags,
    ) -> HQResult<()> {
        if steps.try_borrow()?.contains_key(&step) {
            return Ok(());
        }
        let step_func = StepFunc::new(registries, Rc::clone(&steps), flags);
        let mut instrs = vec![];
        let mut type_stack = vec![];
        for opcode in step.opcodes() {
            let inputs = type_stack
                .splice((type_stack.len() - opcode.acceptable_inputs().len()).., [])
                .collect();
            instrs.append(&mut wrap_instruction(
                &step_func,
                Rc::clone(&inputs),
                opcode.clone(),
            )?);
            if let Some(output) = opcode.output_type(inputs)? {
                type_stack.push(output);
            }
        }
        step_func.add_instructions(instrs)?;
        steps.try_borrow_mut()?.insert(step, step_func);
        Ok(())
    }
}
