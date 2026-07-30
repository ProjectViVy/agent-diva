use agent_diva_core::bus::PlanRuntimeState;
use agent_diva_core::planning::model::PlanPhase;

use super::super::policy_phase_for;

/// Immutable policy-relevant decisions for one sampling boundary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TurnSnapshot {
    pub session_key: String,
    pub model: String,
    pub plan_mode: bool,
    pub policy_phase: Option<PlanPhase>,
    pub reviewer_read_only: bool,
    pub scheduled: bool,
    pub trace_id: String,
}

impl TurnSnapshot {
    pub(crate) fn capture(
        session_key: String,
        model: String,
        plan_mode: bool,
        active_plan: Option<&PlanRuntimeState>,
        reviewer_read_only: bool,
        scheduled: bool,
        trace_id: String,
    ) -> Self {
        Self {
            session_key,
            model,
            plan_mode,
            policy_phase: policy_phase_for(active_plan, plan_mode),
            reviewer_read_only,
            scheduled,
            trace_id,
        }
    }

    /// Refresh the phase after runtime-control or planning-tool state changes.
    pub(crate) fn refresh_policy(&mut self, active_plan: Option<&PlanRuntimeState>) {
        self.policy_phase = policy_phase_for(active_plan, self.plan_mode);
    }

    pub(crate) fn plan_guard_active(&self) -> bool {
        self.policy_phase.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_mode_snapshot_fails_closed_without_persisted_plan() {
        let snapshot = TurnSnapshot::capture(
            "gui:chat".into(),
            "model".into(),
            true,
            None,
            true,
            false,
            "trace".into(),
        );
        assert_eq!(snapshot.policy_phase, Some(PlanPhase::Plan));
        assert!(snapshot.plan_guard_active());
        assert!(snapshot.reviewer_read_only);
        assert!(!snapshot.scheduled);
    }
}
