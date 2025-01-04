use super::{ProcedureContext, Target};
use crate::prelude::*;

#[derive(Debug, Clone)]
pub struct StepContext {
    pub target: Weak<Target>,
    pub proc_context: Option<ProcedureContext>,
}
