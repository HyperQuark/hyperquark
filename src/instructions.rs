//! Contains information about instructions (roughly anaologus to blocks),
//! including input type validation, output type mapping, and WASM generation.

#![allow(
    clippy::unnecessary_wraps,
    reason = "many functions here needlessly return `Result`s in order to keep type signatures consistent"
)]
#![allow(
    clippy::needless_pass_by_value,
    reason = "there are so many `Rc<T>`s here which I don't want to change"
)]

use crate::prelude::*;

mod control;
mod data;
mod hq;
mod looks;
mod motion;
mod operator;
mod pen;
mod procedures;
mod sensing;

#[macro_use]
mod tests;

fn boxed_output_type<F>(
    // we provide a function so this can be used by tests without access to the whole IrOpcode enum stuff
    outputs_func: F,
    inputs: Rc<[crate::ir::Type]>,
) -> HQResult<crate::ir::ReturnType>
where
    F: Fn(Rc<[crate::ir::Type]>) -> HQResult<crate::ir::ReturnType>,
{
    use crate::ir::ReturnType;
    if inputs.is_empty() {
        let out = outputs_func(Rc::from([]))?;
        // crate::log!("{out:?}");
        Ok(out)
    } else {
        // crate::log!("{inputs:?}");
        if inputs.iter().any(crate::ir::Type::is_none) {
            hq_bug!("got none input type :scream:")
        }
        let bases = crate::ir::base_types(&inputs)?;
        let mapped = bases
            .iter()
            .enumerate()
            .map(|(i, tys)| {
                let input = inputs[i];
                tys.iter().map(move |ty| ty.and(input))
            })
            .collect::<Box<[_]>>();
        // crate::log!("{mapped:?}");
        let inins = mapped
            .iter()
            .cloned()
            .multi_cartesian_product()
            .map(|ins| outputs_func(ins.into_iter().collect()))
            .collect::<Box<[_]>>();
        // crate::log!("{:?}", inins);
        inins
            .iter()
            .cloned()
            .try_reduce(|acc, el| {
                #[expect(clippy::redundant_clone, reason = "false positives")]
                Ok(match acc? {
                    ReturnType::None => {
                        hq_assert!(matches!(el.clone()?, ReturnType::None));
                        Ok(ReturnType::None)
                    }
                    ReturnType::Singleton(ty) => {
                        if let ReturnType::Singleton(ty2) = el.clone()? {
                            Ok(ReturnType::Singleton(ty.or(ty2)))
                        } else {
                            hq_bug!("")
                        }
                    }
                    ReturnType::MultiValue(tys) => {
                        let ReturnType::MultiValue(tys2) = el.clone()? else {
                            hq_bug!("")
                        };
                        hq_assert_eq!(tys.len(), tys2.len());
                        Ok(ReturnType::MultiValue(
                            tys.iter()
                                .zip(tys2.iter())
                                .map(|(ty1, ty2)| ty1.or(*ty2))
                                .collect(),
                        ))
                    }
                })
            })?
            .ok_or_else(|| make_hq_bug!(""))?
    }
}

include!(concat!(env!("OUT_DIR"), "/ir-opcodes.rs"));

pub mod input_switcher;
pub use input_switcher::wrap_instruction;

pub use hq::r#yield::YieldMode;

mod prelude {
    pub use crate::ir::{ReturnType, Type as IrType};
    pub use crate::prelude::*;
    pub use crate::wasm::{InternalInstruction, StepFunc};
    pub use ReturnType::{MultiValue, Singleton};
    pub use wasm_encoder::{RefType, ValType};
    pub use wasm_gen::wasm;

    /// Canonical NaN + bit 33, + string pointer in bits 1-32
    pub const BOXED_STRING_PATTERN: i64 = 0x7FF8_0001 << 32;
    /// Canonical NaN + bit 34, + i32 in bits 1-32
    pub const BOXED_INT_PATTERN: i64 = 0x7ff8_0002 << 32;
    /// Canonical NaN + bit 35, + i32 in bits 1-32
    pub const BOXED_COLOR_RGB_PATTERN: i64 = 0x7ff8_0004 << 32;
    /// Canonical NaN + bit 36, + i32 in bits 1-32
    pub const BOXED_COLOR_ARGB_PATTERN: i64 = 0x7ff8_0008 << 32;
}
