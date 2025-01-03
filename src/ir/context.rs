use super::ProcedureContext;

#[derive(Debug, Clone, Copy)]
pub struct TargetContext {
    pub target: u32,
}
#[derive(Debug, Clone)]
pub struct StepContext {
    pub target_context: TargetContext,
    pub proc_context: Option<ProcedureContext>,
}
