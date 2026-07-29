use agent_diva_core::planning::model::PlanPhase;

/// Policy context required immediately before the single tool execution seam.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ToolStepPolicy {
    pub phase: Option<PlanPhase>,
    pub cancelled: bool,
}

impl ToolStepPolicy {
    pub(crate) fn may_enter_executor(&self) -> bool {
        !self.cancelled
    }
}
