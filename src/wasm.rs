use crate::ir::types::Type as IrType;
use crate::prelude::*;
use wasm_encoder::{Function, Instruction, ValType};

pub mod locals {
    pub const MEM_LOCATION: u32 = 0;
    pub const EXTERNREF: u32 = 1;
    pub const F64: u32 = 2;
    pub const I64: u32 = 3;
    pub const I32: u32 = 4;
    pub const I32_2: u32 = 5;
    pub const F64_2: u32 = 6;
}

pub struct StepFunc {
    locals: RefCell<Vec<ValType>>,
    instructions: RefCell<Vec<Instruction<'static>>>,
    param_count: u32,
}

impl StepFunc {
    pub fn new() -> Self {
        StepFunc {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            param_count: 1,
        }
    }

    pub fn new_with_param_count(count: usize) -> HQResult<Self> {
        Ok(StepFunc {
            locals: RefCell::new(vec![]),
            instructions: RefCell::new(vec![]),
            param_count: u32::try_from(count)
                .map_err(|_| make_hq_bug!("param count out of bounds"))?,
        })
    }

    pub fn get_local(&self, val_type: ValType, count: u32) -> HQResult<u32> {
        let existing_count = self
            .locals
            .borrow()
            .iter()
            .filter(|ty| **ty == val_type)
            .count();
        Ok(u32::try_from(if existing_count < (count as usize) {
            {
                self.locals
                    .borrow_mut()
                    .extend([val_type].repeat(count as usize - existing_count));
            }
            self.locals.borrow().len() - 1
        } else {
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

pub fn ir_type_to_wasm(ir_type: IrType) -> HQResult<ValType> {
    Ok(if IrType::Float.contains(ir_type) {
        ValType::F64
    } else if IrType::QuasiInt.contains(ir_type) {
        ValType::I64
    } else {
        hq_todo!()
    })
}

pub struct WasmProject {
    step_funcs: RefCell<Vec<StepFunc>>,
}

impl WasmProject {
    pub fn add_step_func(&self, step_func: StepFunc) {
        self.step_funcs.borrow_mut().push(step_func);
    }

    pub fn finish(self) -> HQResult<Vec<u8>> {
        hq_todo!();
    }
}
