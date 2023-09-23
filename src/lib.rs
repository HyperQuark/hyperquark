#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate alloc;
extern crate enum_field_getter;

use alloc::string::{String, ToString};

use wasm_bindgen::prelude::*;

pub mod ir;
pub mod sb3;
pub mod targets;

use targets::wasm;

#[wasm_bindgen]
pub fn sb3_to_wasm(proj: &str) -> wasm::WasmProject {
    ir::IrProject::from(sb3::Sb3Project::try_from(proj).expect("uh oh")).into()
}
