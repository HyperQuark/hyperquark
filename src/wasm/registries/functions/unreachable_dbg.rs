use wasm_encoder::{FieldType, HeapType, RefType, StorageType, ValType};
use wasm_gen::wasm_const;

use super::{MaybeStaticFunction, StaticFunction};
use crate::prelude::*;

/// Traps. Used for exposing the wasm module in devtools.
pub struct UnreachableDbg;
impl NamedRegistryItem<MaybeStaticFunction> for UnreachableDbg {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        register_deps: || vec![],
        maybe_populate: |_, _| {
            Ok(Some(StaticFunction {
                export: Some("unreachable_dbg".into()),
                instructions: Box::from(wasm_const![Unreachable, End] as &[_]),
                params: Box::new([]),
                returns: Box::new([]),
                locals: Box::new([]),
            }))
        },
    };
}
