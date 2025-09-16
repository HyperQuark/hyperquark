use super::{IrProject, ProcContext, Target};
use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct StepContext {
    pub target: Rc<Target>,
    /// whether or not the current thread is warped. this may be because the current
    /// procedure is warped, or because a procedure higher up the call stack was warped.
    pub warp: bool,
    pub proc_context: Option<ProcContext>,
    /// enables certain behaviours such as `console.log` say/think rather than
    /// displaying in bubbles
    pub debug: bool,
}

impl StepContext {
    pub fn project(&self) -> HQResult<Rc<IrProject>> {
        self.target()
            .project()
            .upgrade()
            .ok_or_else(|| make_hq_bug!("couldn't upgrade Weak"))
    }

    #[must_use]
    pub const fn target(&self) -> &Rc<Target> {
        &self.target
    }
}
