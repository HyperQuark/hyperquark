use crate::prelude::*;
use crate::registry::SetRegistry;

pub type StringRegistry = SetRegistry<Box<str>>;

impl StringRegistry {
    pub fn finish(self) -> Vec<String> {
        self.registry()
            .take()
            .keys()
            .cloned()
            .map(|s| s.into_string())
            .collect()
    }
}
