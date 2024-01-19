#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate alloc;
extern crate enum_field_getter;

use wasm_bindgen::prelude::*;

#[macro_use]
mod error;
pub mod ir;
pub mod sb3;
pub mod targets;

pub use error::{HQError, HQErrorType};

use targets::wasm;

#[wasm_bindgen]
pub fn sb3_to_wasm(proj: &str) -> Result<wasm::WasmProject, HQError> {
    ir::IrProject::from(sb3::Sb3Project::try_from(proj)?).try_into()
}
