use super::{IrProject, ProcContext, Target};
use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct StepContext {
    pub target: Weak<Target>,
    /// whether or not the current thread is warped. this may be because the current
    /// procedure is warped, or because a procedure higher up the call stack was warped.
    pub warp: bool,
    pub proc_context: Option<ProcContext>,
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
