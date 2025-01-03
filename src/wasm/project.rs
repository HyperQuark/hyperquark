use super::ExternalEnvironment;
use crate::ir::{IrProject, Type as IrType};
use crate::prelude::*;
use crate::wasm::{StepFunc, WasmFlags};
use wasm_encoder::ValType;

/// A respresntation of a WASM representation of a project. Cannot be created directly;
/// use `TryFrom<IrProject>`.
pub struct WasmProject {
    flags: WasmFlags,
    step_funcs: Box<[StepFunc]>,
    environment: ExternalEnvironment,
}

impl WasmProject {
    pub fn new(flags: WasmFlags, environment: ExternalEnvironment) -> Self {
        WasmProject {
            flags,
            step_funcs: Box::new([]),
            environment,
        }
    }

    pub fn flags(&self) -> &WasmFlags {
        &self.flags
    }

    pub fn environment(&self) -> ExternalEnvironment {
        self.environment
    }

    pub fn step_funcs(&self) -> &[StepFunc] {
        self.step_funcs.borrow()
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
            hq_todo!() //ValType::V128 // f32x4
        } else {
            ValType::I64 // NaN boxed value... let's worry about colors later
        })
    }

    pub fn finish(self) -> HQResult<Vec<u8>> {
        hq_todo!()
    }
}

impl TryFrom<IrProject> for WasmProject {
    type Error = HQError;

    fn try_from(ir_project: IrProject) -> HQResult<WasmProject> {
        let mut steps: Vec<StepFunc> = vec![];
        for thread in ir_project.threads() {
            
        }
        hq_todo!()
    }
}