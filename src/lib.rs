#![feature(stmt_expr_attributes)]
#![feature(if_let_guard)]
#![feature(let_chains)]
#![feature(try_blocks)]
#![doc(html_logo_url = "https://hyperquark.github.io/hyperquark/logo.png")]
#![doc(html_favicon_url = "https://hyperquark.github.io/hyperquark/favicon.ico")]
#![warn(clippy::cargo, clippy::nursery, clippy::pedantic)]
#![allow(
    clippy::non_std_lazy_statics,
    reason = "bug in clippy (https://github.com/rust-lang/rust-clippy/issues/14729)"
)]
#![allow(
    clippy::missing_errors_doc,
    reason = "Too many Results everywhere to document every possible error case. Errors should be self-descriptive and user readable anyway."
)]
#![allow(clippy::too_many_arguments, reason = "unavoidable at this stage")]
#![allow(
    clippy::trivially_copy_pass_by_ref,
    reason = "too many false positives on WasmFlags, which will grow in future"
)]
#![deny(clippy::allow_attributes, clippy::allow_attributes_without_reason)]
#![warn(
    clippy::alloc_instead_of_core,
    clippy::clone_on_ref_ptr,
    clippy::dbg_macro,
    clippy::expect_used,
    clippy::get_unwrap,
    clippy::missing_asserts_for_indexing,
    clippy::panic,
    clippy::rc_buffer,
    clippy::redundant_type_annotations,
    clippy::shadow_reuse,
    clippy::std_instead_of_alloc,
    clippy::std_instead_of_core,
    clippy::string_to_string,
    clippy::unwrap_used,
    clippy::wildcard_enum_match_arm
)]

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

mod rc;

/// commonly used _things_ which would be nice not to have to type out every time
pub mod prelude {
    pub use crate::registry::{Registry, RegistryDefault};
    pub use crate::{HQError, HQResult};
    pub use alloc::borrow::Cow;
    pub use alloc::boxed::Box;
    pub use alloc::collections::{BTreeMap, BTreeSet};
    pub use alloc::string::{String, ToString};
    pub use alloc::vec::Vec;
    pub use core::borrow::Borrow;
    pub use core::cell::RefCell;
    pub use core::fmt;
    pub use core::marker::PhantomPinned;
    pub use core::pin::Pin;

    pub use crate::rc::{Rc, Weak};

    use core::hash::BuildHasherDefault;
    use hashers::fnv::FNV1aHasher64;
    use indexmap;
    pub type IndexMap<K, V> = indexmap::IndexMap<K, V, BuildHasherDefault<FNV1aHasher64>>;
    pub type IndexSet<T> = indexmap::IndexSet<T, BuildHasherDefault<FNV1aHasher64>>;

    pub use itertools::Itertools;
}

#[cfg(target_family = "wasm")]
#[wasm_bindgen(js_namespace=console)]
extern "C" {
    pub fn log(s: &str);
    pub fn warn(s: &str);
}

#[cfg(not(target_family = "wasm"))]
pub fn log(s: &str) {
    println!("{s}");
}

#[macro_export]
macro_rules! log {
    ($($args:tt)+) => {{
        $crate::log(format!($($args)+).as_str());
    }}
}

#[cfg(not(target_family = "wasm"))]
pub fn warn(s: &str) {
    println!("{s}");
}

#[macro_export]
macro_rules! warn {
    ($($args:tt)+) => {{
        $crate::warn(format!($($args)+).as_str());
    }}
}

#[cfg(feature = "compiler")]
#[wasm_bindgen]
pub fn sb3_to_wasm(proj: &str, flags: wasm::WasmFlags) -> HQResult<wasm::FinishedWasm> {
    use ir::IrProject;
    use wasm::flags::PrintIR;

    let sb3_proj = sb3::Sb3Project::try_from(proj)?;
    let ir_proj = IrProject::try_from_sb3(&sb3_proj, &flags)?;
    if flags.print_ir == PrintIR::On {
        crate::log("ir (before optimisation):");
        crate::log(format!("{ir_proj}").as_str());
    }
    optimisation::ir_optimise(&ir_proj)?;
    if flags.print_ir == PrintIR::On {
        crate::log("ir (after optimisation):");
        crate::log(format!("{ir_proj}").as_str());
    }
    wasm::WasmProject::from_ir(&ir_proj, flags)?.finish()
}
