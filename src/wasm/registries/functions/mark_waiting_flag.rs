use wasm_encoder::{FieldType, HeapType, RefType, StorageType, ValType};
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
        register_deps: |_| Ok(()),
        maybe_populate: |proj, _| {
            let i8_struct_ty = proj.registries().types().struct_(vec![FieldType {
                element_type: StorageType::I8,
                mutable: true,
            }])?;
            Ok(Some(StaticFunction {
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
            }))
        },
    };
}
