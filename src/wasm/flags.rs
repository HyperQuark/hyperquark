use crate::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[non_exhaustive]
#[derive(Copy, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub enum WasmStringType {
    /// externref strings - this will automatically use the JS string builtins proposal if available
    ExternRef,
    Manual,
}

impl Default for WasmStringType {
    fn default() -> Self {
        Self::ExternRef
    }
}

/// compilation flags
#[non_exhaustive]
#[derive(Default, Copy, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct WasmFlags {
    pub string_type: WasmStringType,
}

#[wasm_bindgen]
impl WasmFlags {
    #[wasm_bindgen]
    pub fn from_js(js: JsValue) -> HQResult<WasmFlags> {
        serde_wasm_bindgen::from_value(js)
            .map_err(|_| make_hq_bug!("couldn't convert JsValue to WasmFlags"))
    }

    #[wasm_bindgen]
    pub fn to_js(&self) -> HQResult<JsValue> {
        serde_wasm_bindgen::to_value(&self)
            .map_err(|_| make_hq_bug!("couldn't convert WasmFlags to JsValue"))
    }

    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmFlags {
        Default::default()
    }
}
