#![allow(clippy::cast_possible_wrap, reason = "can't use try_into in const")]

mod dyn_array;
mod mark_waiting_flag;
mod pen_colour;
mod spawn_threads;
mod tick;
mod unreachable_dbg;

use wasm_encoder::{
    CodeSection, EntityType, ExportKind, ExportSection, Function, FunctionSection, ImportSection,
    Instruction as WInstruction, ValType,
};

use super::super::WasmProject;
use super::TypeRegistry;
use crate::prelude::*;
use crate::registry::{MapRegistry, Registry};

pub type ExternalFunctionRegistry =
    MapRegistry<(&'static str, Box<str>), (Vec<ValType>, Vec<ValType>)>;

impl ExternalFunctionRegistry {
    pub fn finish(self, imports: &mut ImportSection, type_registry: &TypeRegistry) -> HQResult<()> {
        for ((module, name), (params, results)) in self.registry().take() {
            let type_index = type_registry.function(params, results)?;
            imports.import(module, &name, EntityType::Function(type_index));
        }
        Ok(())
    }
}

#[derive(Clone)]
pub struct StaticFunction {
    pub instructions: Box<[WInstruction<'static>]>,
    pub params: Box<[ValType]>,
    pub returns: Box<[ValType]>,
    pub locals: Box<[ValType]>,
    pub export: Option<Box<str>>,
}

/// A `const`-able representation of a static function.
///
/// `maybe_populate` should return a `Some(StaticFunction)` if the function instructions
/// are known at compile-time;
/// `static_function` should be overriden to a `Some(StaticFunction)` if the function
/// is overriden.
///
/// It is not possible to populate `static_function` in a `const` context, hence the existence
/// of the `maybe_populate` field.
#[derive(Clone)]
pub struct MaybeStaticFunction {
    pub static_function: Option<StaticFunction>,
    pub maybe_populate: fn(
        &WasmProject,
        &IndexMap<Box<str>, MaybeStaticFunction>,
    ) -> HQResult<Option<StaticFunction>>,
    pub register_deps: fn() -> Vec<(Box<str>, MaybeStaticFunction)>,
}

pub struct StaticFunctionRegistrar;
impl RegistryType for StaticFunctionRegistrar {
    type Value = MaybeStaticFunction;
}
pub type StaticFunctionRegistry = NamedRegistry<StaticFunctionRegistrar>;

impl StaticFunctionRegistry {
    /// Finishes the registry. Returns the final size of the registry.
    pub fn finish(
        self,
        wasm_proj: &WasmProject,
        functions: &mut FunctionSection,
        exports: &mut ExportSection,
        codes: &mut CodeSection,
        type_registry: &TypeRegistry,
        imported_func_count: u32,
    ) -> HQResult<u32> {
        let mut num_funcs = self.registry().borrow().len();
        let mut to_register = vec![];
        loop {
            for (_name, MaybeStaticFunction { register_deps, .. }) in
                self.registry().borrow().iter()
            {
                to_register.extend(register_deps());
            }
            for (key, val) in core::mem::take(&mut to_register) {
                self.register_dyn::<usize>(key, val)?;
            }
            let new_num_funcs = self.registry().borrow().len();
            if new_num_funcs == num_funcs {
                break;
            }
            num_funcs = new_num_funcs;
        }
        let registry = self.registry().take();
        for (
            _name,
            MaybeStaticFunction {
                static_function,
                maybe_populate,
                ..
            },
        ) in &registry
        {
            let Some(StaticFunction {
                instructions,
                params,
                returns,
                locals,
                export,
            }) = static_function
                .clone()
                .map_or_else(|| maybe_populate(wasm_proj, &registry), |sf| Ok(Some(sf)))?
            else {
                hq_bug!(
                    "static functions must either be overriden, or have a non-None maybe_populate \
                     field"
                )
            };
            let type_index = type_registry.function(params.into(), returns.into())?;
            functions.function(type_index);
            let mut func = Function::new_with_locals_types(locals.iter().copied());
            for instruction in instructions {
                func.instruction(&instruction);
            }
            codes.function(&func);
            if let Some(export_name) = export {
                exports.export(
                    &export_name,
                    ExportKind::Func,
                    imported_func_count + functions.len() - 1,
                );
            }
        }
        Ok(num_funcs as u32)
    }
}

pub mod static_functions {
    pub use super::dyn_array::{
        DynArrayClear, DynArrayFuncOverride, DynArrayGet, DynArrayLen, DynArrayNew, DynArrayPop,
        DynArrayPush,
    };
    pub use super::mark_waiting_flag::MarkWaitingFlag;
    pub use super::pen_colour::{UpdatePenColorFromHSV, UpdatePenColorFromRGB};
    pub use super::spawn_threads::{SpawnNewThread, SpawnThreadInStack};
    pub use super::tick::Tick;
    pub use super::unreachable_dbg::UnreachableDbg;
}
