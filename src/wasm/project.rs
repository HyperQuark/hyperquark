use super::ExternalEnvironment;
use crate::ir::Type as IrType;
use crate::prelude::*;
use crate::wasm::{StepFunc, WasmFlags};
use wasm_encoder::ValType;

pub struct WasmProject {
    flags: WasmFlags,
    step_funcs: RefCell<Vec<StepFunc>>,
    environment: ExternalEnvironment,
}

impl WasmProject {
    pub fn new(flags: WasmFlags, environment: ExternalEnvironment) -> Self {
        WasmProject {
            flags,
            step_funcs: RefCell::new(vec![]),
            environment,
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
            hq_todo!(); //ValType::V128 // f32x4
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
