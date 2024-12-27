#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate alloc;
extern crate enum_field_getter;

use wasm_bindgen::prelude::*;

#[macro_use]
mod error;
pub mod ir;
// pub mod ir_opt;
pub mod sb3;
// pub mod wasm;
#[macro_use]
pub mod instructions;

pub use error::{HQError, HQErrorType, HQResult};

// use wasm::wasm;

#[cfg(not(test))]
#[wasm_bindgen(js_namespace=console)]
extern "C" {
    pub fn log(s: &str);
}

#[cfg(test)]
pub fn log(s: &str) {
    println!("{s}")
}

// #[wasm_bindgen]
// pub fn sb3_to_wasm(proj: &str) -> Result<wasm::WasmProject, HQError> {
//     let mut ir_proj = ir::IrProject::try_from(sb3::Sb3Project::try_from(proj)?)?;
//     ir_proj.optimise()?;
//     ir_proj.try_into()
// }

/// commonly used _things_ which would be nice not to have to type out every time
pub mod prelude {
    pub use crate::{HQError, HQResult};
    pub use alloc::boxed::Box;
    pub use alloc::collections::BTreeMap;
    pub use alloc::rc::Rc;
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
    pub use core::cell::RefCell;
    pub use core::fmt;
}