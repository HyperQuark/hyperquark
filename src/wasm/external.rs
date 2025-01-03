use super::TypeRegistry;
use crate::prelude::*;
use wasm_encoder::{EntityType, ImportSection, ValType};

pub type FunctionMap = IndexMap<(&'static str, &'static str), (Vec<ValType>, Vec<ValType>)>;

#[derive(Clone, Default)]
pub struct ExternalFunctionMap(RefCell<FunctionMap>);

impl ExternalFunctionMap {
    pub fn new() -> Self {
        ExternalFunctionMap(RefCell::new(Default::default()))
    }

    pub(crate) fn get_map(&self) -> &RefCell<FunctionMap> {
        &self.0
    }

    /// get the index of the specified function, inserting it if it doesn't exist in the map already.
    /// Doesn't check if the provided params/results types match what's already there.
    pub fn function_index(
        &self,
        module: &'static str,
        name: &'static str,
        params: Vec<ValType>,
        results: Vec<ValType>,
    ) -> HQResult<u32> {
        self.get_map()
            .borrow_mut()
            .entry((module, name))
            .or_insert((params, results));
        u32::try_from(
            self.get_map()
                .borrow()
                .get_index_of(&(module, name))
                .ok_or(make_hq_bug!("couldn't find entry in ExternalFunctionMap"))?,
        )
        .map_err(|_| make_hq_bug!("external function index out of bounds"))
    }

    pub fn finish(
        self,
        imports: &mut ImportSection,
        type_registry: Rc<TypeRegistry>,
    ) -> HQResult<()> {
        for ((module, name), (params, results)) in self.get_map().take() {
            let type_index = type_registry.type_index(params, results)?;
            imports.import(module, name, EntityType::Function(type_index));
        }
        Ok(())
    }
}

#[derive(Debug, Copy, Clone)]
#[non_exhaustive]
pub enum ExternalEnvironment {
    WebBrowser,
}
