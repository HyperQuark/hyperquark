#![cfg_attr(target_family = "wasm", no_std)]
#![doc(html_logo_url = "https://hyperquark.github.io/hyperquark/logo.png")]
#![doc(html_favicon_url = "https://hyperquark.github.io/hyperquark/favicon.ico")]
#![allow(clippy::new_without_default)]

#[macro_use]
extern crate alloc;
extern crate enum_field_getter;

#[cfg(target_family = "wasm")]
use wasm_bindgen::prelude::*;

#[macro_use]
mod error;
pub mod ir;
// pub mod ir_opt;
pub mod sb3;
pub mod wasm;
#[macro_use]
pub mod instructions;

#[doc(inline)]
pub use error::{HQError, HQErrorType, HQResult};

/// commonly used _things_ which would be nice not to have to type out every time
pub mod prelude {
    pub use crate::{HQError, HQResult};
    pub use alloc::boxed::Box;
    pub use alloc::collections::BTreeMap;
    pub use alloc::rc::{Rc, Weak};
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
    pub use core::borrow::Borrow;
    pub use core::cell::RefCell;
    pub use core::fmt;

    use core::hash::BuildHasherDefault;
    use hashers::fnv::FNV1aHasher64;
    use indexmap;
    pub type IndexMap<K, V> = indexmap::IndexMap<K, V, BuildHasherDefault<FNV1aHasher64>>;
    pub type IndexSet<T> = indexmap::IndexSet<T, BuildHasherDefault<FNV1aHasher64>>;
}

use prelude::*;

// use wasm::wasm;

#[cfg(target_family = "wasm")]
#[cfg_attr(target_family = "wasm", wasm_bindgen(js_namespace=console))]
extern "C" {
    pub fn log(s: &str);
}

#[cfg(not(target_family = "wasm"))]
pub fn log(s: &str) {
    println!("{s}")
}

#[cfg_attr(target_family = "wasm", wasm_bindgen)]
pub fn sb3_to_wasm(proj: &str) -> HQResult<Box<[u8]>> {
    let sb3_proj = sb3::Sb3Project::try_from(proj)?;
    let ir_proj: Rc<ir::IrProject> = sb3_proj.try_into()?;
    Ok(wasm::WasmProject::try_from(ir_proj)?
        .finish()?
        .into_boxed_slice())
}
