#![cfg_attr(not(test), no_std)]
#![recursion_limit = "256"]

#[macro_use]
extern crate alloc;
extern crate enum_field_getter;

pub mod sb3;
pub mod targets;
pub mod ir;

extern "C" {
  #[no_mangle]
  pub use targets::wasm::WebWasmFile;
  #[no_mangle]
  pub use sb3::Sb3Project;
}