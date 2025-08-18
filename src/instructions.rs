//! Contains information about instructions (roughly anaologus to blocks),
//! including input type validation, output type mapping, and WASM generation.

#![allow(
    clippy::unnecessary_wraps,
    reason = "many functions here needlessly return `Result`s in order to keep type signatures consistent"
)]
#![allow(
    clippy::needless_pass_by_value,
    reason = "there are so many `Rc<T>`s here which I don't want to change"
)]

use crate::prelude::*;

mod control;
mod data;
mod hq;
mod looks;
mod operator;
mod procedures;
mod sensing;

#[macro_use]
mod tests;

include!(concat!(env!("OUT_DIR"), "/ir-opcodes.rs"));

mod input_switcher;
pub use input_switcher::wrap_instruction;

pub use hq::r#yield::YieldMode;

mod prelude {
    pub use crate::ir::{Type as IrType, ReturnType};
    pub use ReturnType::{MultiValue, Singleton};
    pub use crate::prelude::*;
    pub use crate::wasm::{InternalInstruction, StepFunc};
    pub use wasm_encoder::{RefType, ValType};
    pub use wasm_gen::wasm;

    /// Canonical NaN + bit 33, + string pointer in bits 1-32
    pub const BOXED_STRING_PATTERN: i64 = 0x7FF8_0001 << 32;
    /// Canonical NaN + bit 33, + i32 in bits 1-32
    pub const BOXED_INT_PATTERN: i64 = 0x7ff8_0002 << 32;
}
