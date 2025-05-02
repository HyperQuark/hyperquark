#![doc(html_logo_url = "https://hyperquark.github.io/hyperquark/logo.png")]
#![doc(html_favicon_url = "https://hyperquark.github.io/hyperquark/favicon.ico")]
#![allow(clippy::new_without_default)]

#[macro_use]
extern crate alloc;
extern crate enum_field_getter;

use wasm_bindgen::prelude::*;

#[macro_use]
mod error;
mod ir;
mod optimisation;
// pub mod ir_opt;
mod sb3;
mod wasm;
#[macro_use]
mod instructions;

#[doc(inline)]
pub use error::{HQError, HQErrorType, HQResult};

mod registry;

/// commonly used _things_ which would be nice not to have to type out every time
pub mod prelude {
    pub use crate::registry::{Registry, RegistryDefault};
    pub use crate::{HQError, HQResult};
    pub use alloc::borrow::{Borrow, Cow};
    pub use alloc::boxed::Box;
    pub use alloc::collections::{BTreeMap, BTreeSet};
    pub use alloc::rc::{Rc, Weak};
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
    pub use core::cell::RefCell;
    pub use core::fmt;

    use core::hash::BuildHasherDefault;
    use hashers::fnv::FNV1aHasher64;
    use indexmap;
    pub type IndexMap<K, V> = indexmap::IndexMap<K, V, BuildHasherDefault<FNV1aHasher64>>;
    pub type IndexSet<T> = indexmap::IndexSet<T, BuildHasherDefault<FNV1aHasher64>>;

    pub use itertools::Itertools;
}

use prelude::*;

// use wasm::wasm;

#[cfg(target_family = "wasm")]
#[wasm_bindgen(js_namespace=console)]
extern "C" {
    pub fn log(s: &str);
}

#[cfg(not(target_family = "wasm"))]
pub fn log(s: &str) {
    println!("{s}")
}

#[cfg(feature = "compiler")]
#[wasm_bindgen]
pub fn sb3_to_wasm(proj: &str, flags: wasm::WasmFlags) -> HQResult<wasm::FinishedWasm> {
    let sb3_proj = sb3::Sb3Project::try_from(proj)?;
    let ir_proj = sb3_proj.try_into()?;
    optimisation::ir_optimise(Rc::clone(&ir_proj))?;
    wasm::WasmProject::from_ir(ir_proj, flags)?.finish()
}
