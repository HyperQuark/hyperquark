#![allow(clippy::enum_glob_use, reason = "easier and little risk of pollution")]

use crate::prelude::*;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Copy, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
pub enum WasmStringType {
    ExternRef,
    JsStringBuiltins,
    //Manual,
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[wasm_bindgen]
pub enum Switch {
    On,
    Off,
}

#[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[wasm_bindgen]
pub enum ListType {
    GCArray,
    LinearMemory,
}

// #[derive(Copy, Clone, Serialize, Deserialize, PartialEq, Eq)]
// #[wasm_bindgen]
// pub enum MemoryLayout {
//     Contiguous,
//     MultiMemory,
// }

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[wasm_bindgen]
pub enum WasmFeature {
    ReferenceTypes,
    TypedFunctionReferences,
    JSStringBuiltins,
    BulkMemory,
    GC,
    MultiMemory,
}

#[wasm_bindgen]
#[must_use]
pub fn all_wasm_features() -> Vec<WasmFeature> {
    use WasmFeature::*;
    vec![
        ReferenceTypes,
        TypedFunctionReferences,
        JSStringBuiltins,
        BulkMemory,
        GC,
        MultiMemory,
    ]
}

#[cfg(test)]
#[must_use]
pub fn unit_test_wasm_features() -> Vec<WasmFeature> {
    use WasmFeature::*;
    vec![
        ReferenceTypes,
        JSStringBuiltins,
        BulkMemory,
        GC,
        MultiMemory,
    ]
}

// no &self because wasm_bidgen doesn't like it
#[wasm_bindgen]
#[must_use]
pub fn wasm_feature_detect_name(feat: WasmFeature) -> String {
    use WasmFeature::*;
    match feat {
        ReferenceTypes => "referenceTypes",
        TypedFunctionReferences => "typedFunctionReferences",
        JSStringBuiltins => "jsStringBuiltins",
        BulkMemory => "bulkMemory",
        GC => "gc",
        MultiMemory => "multiMemory",
    }
    .into()
}

#[derive(Clone, Serialize, Deserialize)]
#[wasm_bindgen(getter_with_clone)]
#[expect(
    clippy::unsafe_derive_deserialize,
    reason = "wasm-bindgen introduces unsafe methods"
)]
pub struct FlagInfo {
    /// a human-readable name for the flag
    pub name: String,
    pub description: String,
    pub ty: String,
    /// which WASM features does this flag rely on?
    wasm_features: BTreeMap<String, Vec<WasmFeature>>,
}

#[wasm_bindgen]
impl FlagInfo {
    fn new() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            ty: String::new(),
            wasm_features: BTreeMap::default(),
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

    fn with_wasm_features(mut self, wasm_features: BTreeMap<String, Vec<WasmFeature>>) -> Self {
        self.wasm_features = wasm_features;
        self
    }

    #[wasm_bindgen]
    #[must_use]
    pub fn wasm_features(&self, flag: &str) -> Option<Vec<WasmFeature>> {
        self.wasm_features.get(flag).cloned()
    }

    #[wasm_bindgen]
    pub fn to_js(&self) -> HQResult<JsValue> {
        serde_wasm_bindgen::to_value(&self)
            .map_err(|_| make_hq_bug!("couldn't convert FlagInfo to JsValue"))
    }
}

macro_rules! stringmap {
    ($($k:ident : $v:expr),+ $(,)?) => {{
        BTreeMap::from([$((String::from(stringify!($k)), $v),)+])
    }}
}

/// stringifies the name of a type whilst ensuring that the type is valid
macro_rules! ty_str {
    ($ty:ty) => {{
        let _ = core::any::TypeId::of::<$ty>(); // forces the type to be valid
        stringify!($ty)
    }};
}

/// compilation flags
#[derive(Copy, Clone, Serialize, Deserialize)]
#[wasm_bindgen]
#[expect(
    clippy::unsafe_derive_deserialize,
    reason = "wasm-bindgen introduces unsafe methods"
)]
pub struct WasmFlags {
    pub string_type: WasmStringType,
    pub wasm_opt: Switch,
    pub print_ir: Switch,
    pub integers: Switch,
    pub list_type: ListType,
    // pub memory_layout: MemoryLayout
}

#[wasm_bindgen]
impl WasmFlags {
    // these attributes should be at the item level, but they don't seem to work there.
    #![expect(
        clippy::needless_pass_by_value,
        reason = "wasm-bindgen does not support &[T]"
    )]

    #[wasm_bindgen]
    pub fn from_js(js: JsValue) -> HQResult<Self> {
        serde_wasm_bindgen::from_value(js)
            .map_err(|_| make_hq_bug!("couldn't convert JsValue to WasmFlags"))
    }

    #[wasm_bindgen]
    pub fn to_js(&self) -> HQResult<JsValue> {
        serde_wasm_bindgen::to_value(&self)
            .map_err(|_| make_hq_bug!("couldn't convert WasmFlags to JsValue"))
    }

    #[wasm_bindgen(constructor)]
    #[must_use]
    pub fn new(wasm_features: Vec<WasmFeature>) -> Self {
        // crate::log(format!("{wasm_features:?}").as_str());
        Self {
            wasm_opt: Switch::On,
            string_type: if wasm_features.contains(&WasmFeature::JSStringBuiltins) {
                WasmStringType::JsStringBuiltins
            } else {
                WasmStringType::ExternRef
            },
            print_ir: Switch::Off,
            integers: Switch::Off,
            list_type: if wasm_features.contains(&WasmFeature::GC) {
                ListType::GCArray
            } else {
                ListType::LinearMemory
            },
        }
    }

    #[wasm_bindgen]
    #[must_use]
    pub fn flag_info(flag: &str) -> FlagInfo {
        match flag {
            "string_type" => FlagInfo::new()
                .with_name("Internal string representation")
                .with_description(
                    "ExternRef - uses JavaScript strings.\
                    <br>\
                    JsStringBuiltins (recommended) - uses JavaScript strings with the JS String Builtins proposal",
                )
                .with_ty(ty_str!(WasmStringType))
                .with_wasm_features(stringmap! {
                    ExternRef : vec![WasmFeature::ReferenceTypes],
                    JsStringBuiltins : vec![WasmFeature::ReferenceTypes, WasmFeature::JSStringBuiltins],
                }),
            "list_type" => FlagInfo::new()
                .with_name("List representation")
                .with_description(
                    "GCArray (recommended) - uses GC arrays.\
                    <br>\
                    LinearMemory (unimplemented) - stores lists in linear memory."
                )
                .with_ty(ty_str!(ListType))
                .with_wasm_features(stringmap! {
                    GCArray : vec![WasmFeature::GC],
                    LinearMemory : vec![]
                }),
            // "memory_layout" => FlagInfo::new()
            //     .with_name("Memory layout")
            //     .with_description(
            //         "Contiguous - stores everything in one contiguous block of memory.\
            //         <br>\
            //         MultiMemory (recommended) - uses multiple memories to avoid reallocations."
            //     )
            //     .with_ty(ty_str!(MemoryLayout))
            //     .with_wasm_features(stringmap! {
            //         Contiguous : vec![],
            //         LinearMemory : vec![WasmFeature::MultiMemory]
            //     }),
            "wasm_opt" => FlagInfo::new()
                .with_name("WASM optimisation")
                .with_description("Should we try to optimise generated WASM modules using wasm-opt?")
                .with_ty(ty_str!(Switch)),
            "print_ir" => FlagInfo::new()
                .with_name("Print IR")
                .with_description("For debugging purposes only")
                .with_ty(ty_str!(Switch)),
            "integers" => FlagInfo::new()
                .with_name("Integers")
                .with_description("Emit integer instructions wherever possible. May make things faster at \
                the cost of possible overflow, or may slow things down if mixed with floats.")
                .with_ty(ty_str!(Switch)),
            _ => FlagInfo::new().with_name(format!("unknown setting '{flag}'").as_str())
        }
    }
}
