//! Contains information about instructions (roughly anaologus to blocks),
//! including input type validation, output type mapping, and WASM generation.

use crate::prelude::*;

mod looks;
mod operator;
mod math;
#[macro_use]
mod utilities;
pub use utilities::{file_block_category, file_block_name, file_opcode};

include!(concat!(env!("OUT_DIR"), "/ir-opcodes.rs"));
