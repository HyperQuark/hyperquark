//! Contains information about instructions (roughly anaologus to blocks),
//! including input type validation, output type mapping, and WASM generation.

use crate::prelude::*;

mod hq;
mod looks;
mod operator;
mod sensing;
#[macro_use]
mod tests;

include!(concat!(env!("OUT_DIR"), "/ir-opcodes.rs"));

mod input_switcher;
pub use input_switcher::wrap_instruction;

mod prelude {
    pub(crate) use crate::ir::Type as IrType;
    pub use crate::prelude::*;
    pub(crate) use crate::wasm::StepFunc;
    pub use wasm_encoder::{Instruction, ValType};
    pub use wasm_gen::wasm;
}
