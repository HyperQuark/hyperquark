use wasm_encoder::{HeapType, RefType, ValType};
use wasm_gen::wasm_const;

use super::{MaybeStaticFunction, StaticFunction};
use crate::prelude::*;

/// Mark a waiting flag as done.
///
/// This is designed to be exported (as `"mark_waiting_flag"`) and called by JS.
///
/// Takes 1 parameter:
/// - A nonnull struct with a single i8 field.
///
/// Override with one u32, the single-field i8 struct type index
pub struct MarkWaitingFlag;
impl NamedRegistryItem<MaybeStaticFunction> for MarkWaitingFlag {
    const VALUE: MaybeStaticFunction = MaybeStaticFunction {
        static_function: None,
        maybe_populate: || None,
    };
}
pub type MarkWaitingFlagOverride = u32;
impl NamedRegistryItemOverride<MaybeStaticFunction, MarkWaitingFlagOverride> for MarkWaitingFlag {
    fn r#override(i8_struct_ty: u32) -> MaybeStaticFunction {
        MaybeStaticFunction {
            static_function: Some(StaticFunction {
                export: Some("mark_waiting_flag".into()),
                instructions: Box::from(wasm_const![
                    LocalGet(0),
                    I32Const(1),
                    StructSet {
                        struct_type_index: i8_struct_ty,
                        field_index: 0
                    },
                    End,
                ] as &[_]),
                params: Box::new([ValType::Ref(RefType {
                    nullable: false,
                    heap_type: HeapType::Concrete(i8_struct_ty),
                })]),
                returns: Box::new([]),
                locals: Box::new([]),
            }),
            maybe_populate: || None,
        }
    }
}
