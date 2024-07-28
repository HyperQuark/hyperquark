#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate alloc;
extern crate enum_field_getter;

use wasm_bindgen::prelude::*;

#[macro_use]
mod error;
pub mod ir;
pub mod ir_opt;
pub mod sb3;
pub mod targets;

pub use error::{HQError, HQErrorType};

use targets::wasm;

#[wasm_bindgen(js_namespace=console)]
extern "C" {
    pub fn log(s: &str);
}

#[wasm_bindgen]
pub fn sb3_to_wasm(proj: &str) -> Result<wasm::WasmProject, HQError> {
    let mut ir_proj = ir::IrProject::try_from(sb3::Sb3Project::try_from(proj)?)?;
    ir_proj.optimise()?;
    ir_proj.try_into()
}
