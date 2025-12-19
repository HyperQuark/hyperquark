pub mod external;
pub mod flags;
pub mod func;
#[macro_use]
pub mod mem_layout;
pub mod project;
pub mod registries;

pub use external::ExternalEnvironment;
pub use flags::WasmFlags;
pub use func::{Instruction as InternalInstruction, StepFunc, StepTarget};
pub use project::{FinishedWasm, WasmProject};
pub use registries::Registries;
pub use registries::{GlobalExportable, GlobalMutable, StepsTable, StringsTable, ThreadsTable};

/// the same as Into, but `const`.
#[must_use]
pub const fn f32_to_ieeef32(f: f32) -> wasm_encoder::Ieee32 {
    unsafe { core::mem::transmute(u32::from_le_bytes(f.to_le_bytes())) }
}
