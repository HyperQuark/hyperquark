use crate::prelude::*;
use wasm_encoder::{TypeSection, ValType};

pub type TypeSet = IndexSet<(Vec<ValType>, Vec<ValType>)>;

#[derive(Clone, Default)]
pub struct TypeRegistry(RefCell<TypeSet>);

impl TypeRegistry {
    pub fn new() -> Self {
        TypeRegistry(RefCell::new(Default::default()))
    }

    pub(crate) fn get_set(&self) -> &RefCell<TypeSet> {
        &self.0
    }

    /// get the index of the specified type, inserting it if it doesn't exist in the set already.
    pub fn type_index(&self, params: Vec<ValType>, results: Vec<ValType>) -> HQResult<u32> {
        self.get_set()
            .borrow_mut()
            .insert((params.clone(), results.clone()));
        u32::try_from(
            self.get_set()
                .borrow()
                .get_index_of(&(params, results))
                .ok_or(make_hq_bug!("couldn't find entry in TypeRegistry"))?,
        )
        .map_err(|_| make_hq_bug!("type index out of bounds"))
    }

    pub fn finish(self, types: &mut TypeSection) {
        for (params, results) in self.get_set().take() {
            types.ty().function(params, results);
        }
    }
}
