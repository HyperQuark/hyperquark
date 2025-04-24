use crate::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Copy, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub enum WasmStringType {
    /// externref strings - this will automatically use the JS string builtins proposal if available
    ExternRef,
    //Manual,
}

impl Default for WasmStringType {
    fn default() -> Self {
        Self::ExternRef
    }
}

/// compilation flags
#[derive(Copy, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct WasmFlags {
    pub string_type: WasmStringType,
    pub wasm_opt: bool,
}

impl Default for WasmFlags {
    fn default() -> WasmFlags {
        WasmFlags {
            wasm_opt: true,
            string_type: Default::default(),
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub enum WasmFeature {
    TypedFunctionReferences,
}

#[derive(Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub struct FlagInfo {
    /// a human-readable name for the flag
    name: String,
    description: String,
    ty: String,
    /// which WASM features does this flag rely on?
    wasm_features: Vec<WasmFeature>,
}

#[wasm_bindgen]
impl FlagInfo {
    fn new() -> Self {
        FlagInfo {
            name: "".into(),
            description: "".into(),
            ty: "".into(),
            wasm_features: vec![],
        }
    }

    fn with_name(mut self, name: &str) -> Self {
        self.name = name.to_string();
        self
    }

    fn with_description(mut self, description: &str) -> Self {
        self.description = description.to_string();
        self
    }

    fn with_ty(mut self, ty: &str) -> Self {
        self.ty = ty.to_string();
        self
    }

    fn with_wasm_features(mut self, wasm_features: Vec<WasmFeature>) -> Self {
        self.wasm_features = wasm_features;
        self
    }

    #[wasm_bindgen]
    pub fn name(&self) -> String {
        self.name.clone()
    }

    #[wasm_bindgen]
    pub fn description(&self) -> String {
        self.description.clone()
    }

    #[wasm_bindgen]
    pub fn ty(&self) -> String {
        self.ty.clone()
    }

    #[wasm_bindgen]
    pub fn wasm_features(&self) -> Vec<WasmFeature> {
        self.wasm_features.clone()
    }

    #[allow(clippy::wrong_self_convention)]
    #[wasm_bindgen]
    pub fn to_js(&self) -> HQResult<JsValue> {
        serde_wasm_bindgen::to_value(&self)
            .map_err(|_| make_hq_bug!("couldn't convert FlagInfo to JsValue"))
    }
}

#[wasm_bindgen]
impl WasmFlags {
    #[wasm_bindgen]
    pub fn from_js(js: JsValue) -> HQResult<WasmFlags> {
        serde_wasm_bindgen::from_value(js)
            .map_err(|_| make_hq_bug!("couldn't convert JsValue to WasmFlags"))
    }

    #[allow(clippy::wrong_self_convention)]
    #[wasm_bindgen]
    pub fn to_js(&self) -> HQResult<JsValue> {
        serde_wasm_bindgen::to_value(&self)
            .map_err(|_| make_hq_bug!("couldn't convert WasmFlags to JsValue"))
    }

    #[wasm_bindgen(constructor)]
    pub fn new() -> WasmFlags {
        Default::default()
    }

    #[wasm_bindgen]
    pub fn flag_info(flag: &str) -> FlagInfo {
        match flag {
            "string_type" => FlagInfo::new()
                .with_name("Internal string representation")
                .with_description(
                    "ExternRef - uses JavaScript strings with JS string builtins where available.",
                )
                .with_ty(stringify!(WasmStringType)),
            "wasm_opt" => FlagInfo::new()
                .with_name("WASM optimisation")
                .with_description("Should we try to optimise generated WASM modules using wasm-opt?")
                .with_ty("boolean"),
            _ => FlagInfo::new(),
        }
        .into()
    }
}
