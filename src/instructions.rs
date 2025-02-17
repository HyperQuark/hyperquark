//! Contains information about instructions (roughly anaologus to blocks),
//! including input type validation, output type mapping, and WASM generation.

use crate::prelude::*;

mod hq;
mod looks;
mod operator;
#[macro_use]
mod tests;

include!(concat!(env!("OUT_DIR"), "/ir-opcodes.rs"));

mod input_switcher;
pub use input_switcher::wrap_instruction;
