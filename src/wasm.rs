use crate::ir::types::Type as IrType;
use crate::prelude::*;
use wasm_encoder::{Function, Instruction, ValType};

/// representation of a step's function
pub struct StepFunc {
    locals: RefCell<Vec<ValType>>,
    instructions: RefCell<Vec<Instruction<'static>>>,
    param_count: u32,
}

impl StepFunc {
    /// creates a new step function, with one paramter
    pub fn new() -> Self {
        StepFunc {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            param_count: 1,
        }
    }
    
    /// creates a new step function with the specified amount of paramters.
    /// currently only used in testing to validate types
    pub fn new_with_param_count(count: usize) -> HQResult<Self> {
        Ok(StepFunc {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            param_count: u32::try_from(count)
                .map_err(|_| make_hq_bug!("param count out of bounds"))?,
        })
    }

    /// Returns the index of the `n`th local of the specified type in this function,
    /// adding some if necessary
    pub fn get_local(&self, val_type: ValType, n: u32) -> HQResult<u32> {
        let existing_count = self
            .locals
            .borrow()
            .iter()
            .filter(|ty| **ty == val_type)
            .count();
        Ok(u32::try_from(if existing_count < (n as usize) {
            {
                self.locals
                    .borrow_mut()
                    .extend([val_type].repeat(n as usize - existing_count));
            }
            self.locals.borrow().len() - 1
        } else {
            // TODO: return nth rather than last
            self.locals
                .borrow()
                .iter()
                .rposition(|ty| *ty == val_type)
                .unwrap()
        })
        .map_err(|_| make_hq_bug!("local index was out of bounds"))?
            + self.param_count)
    }

    pub fn add_instructions(&self, instructions: impl IntoIterator<Item = Instruction<'static>>) {
        self.instructions.borrow_mut().extend(instructions);
    }

    /// Takes ownership of the function and returns the backing `wasm_encoder` `Function`
    pub fn finish(self) -> Function {
        let mut func = Function::new_with_locals_types(self.locals.take());
        for instruction in self.instructions.take() {
            func.instruction(&instruction);
        }
        func.instruction(&Instruction::End);
        func
    }
}

impl Default for StepFunc {
    fn default() -> Self {
        Self::new()
    }
}

#[non_exhaustive]
pub enum WasmStringType {
    ExternRef,
    JsString,
}

impl Default for WasmStringType {
    fn default() -> Self {
        Self::ExternRef
    }
}

/// compilation flags
#[non_exhaustive]
#[derive(Default)]
pub struct WasmFlags {
    pub string_type: WasmStringType,
}

pub struct WasmProject {
    flags: WasmFlags,
    step_funcs: RefCell<Vec<StepFunc>>,
}

impl WasmProject {
    pub fn new(flags: WasmFlags) -> Self {
        WasmProject {
            flags,
            step_funcs: RefCell::new(vec![]),
        }
    }

    pub fn flags(&self) -> &WasmFlags {
        &self.flags
    }

    /// maps a broad IR type to a WASM type
    pub fn ir_type_to_wasm(&self, ir_type: IrType) -> HQResult<ValType> {
        Ok(if IrType::Float.contains(ir_type) {
            ValType::F64
        } else if IrType::QuasiInt.contains(ir_type) {
            ValType::I64
        } else if IrType::String.contains(ir_type) {
            ValType::EXTERNREF
        } else if IrType::Color.contains(ir_type) {
            hq_todo!();//ValType::V128 // f32x4
        } else {
            ValType::I64 // NaN boxed value... let's worry about colors later
        })
    }

    pub fn add_step_func(&self, step_func: StepFunc) {
        self.step_funcs.borrow_mut().push(step_func);
    }

    pub fn finish(self) -> HQResult<Vec<u8>> {
        hq_todo!();
    }
}
