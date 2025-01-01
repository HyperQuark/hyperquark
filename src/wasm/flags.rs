#[non_exhaustive]
pub enum WasmStringType {
    ExternRef,
    JsString,
}

impl Default for WasmStringType {
    fn default() -> Self {
        Self::ExternRef
    }
}

/// compilation flags
#[non_exhaustive]
#[derive(Default)]
pub struct WasmFlags {
    pub string_type: WasmStringType,
}
