#![cfg_attr(not(test), no_std)]
#![recursion_limit = "256"]

#[macro_use]
extern crate alloc;
extern crate enum_field_getter;

use alloc::string::{String, ToString};

use wasm_bindgen::prelude::*;

pub mod sb3;
pub mod targets;
pub mod ir;

#[wasm_bindgen]
pub fn sb3_to_wasm(proj: &str) -> String {
    (*targets::wasm::WebWasmFile::from(sb3::Sb3Project::try_from(proj).expect("uh oh")).js_string().clone()).to_string()
}