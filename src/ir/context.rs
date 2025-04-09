use super::{IrProject, ProcedureContext, Target};
use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct StepContext {
    pub target: Weak<Target>,
    pub proc_context: Option<ProcedureContext>,
}

impl StepContext {
    pub fn project(&self) -> HQResult<Weak<IrProject>> {
        Ok(self
            .target
            .upgrade()
            .ok_or(make_hq_bug!("couldn't upgrade Weak"))?
            .project())
    }
}
