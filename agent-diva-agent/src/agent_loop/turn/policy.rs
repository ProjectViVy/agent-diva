use agent_diva_core::bus::PlanRuntimeState;
use agent_diva_core::planning::model::PlanPhase;

use super::super::policy_phase_for;

/// Turn-local policy decisions and write-preflight state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TurnSnapshot {
    pub session_key: String,
    pub model: String,
    pub plan_mode: bool,
    pub policy_phase: Option<PlanPhase>,
    pub reviewer_read_only: bool,
    pub scheduled: bool,
    pub trace_id: String,
    memory_rules_injected_iteration: Option<usize>,
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
            memory_rules_injected_iteration: None,
        }
    }

    /// Refresh the phase after runtime-control or planning-tool state changes.
    pub(crate) fn refresh_policy(&mut self, active_plan: Option<&PlanRuntimeState>) {
        self.policy_phase = policy_phase_for(active_plan, self.plan_mode);
    }

    pub(crate) fn plan_guard_active(&self) -> bool {
        self.policy_phase.is_some()
    }

    /// Record the provider iteration whose tool results first carried the full
    /// Memory write handbook. Calls from that same provider response must not
    /// execute because the model could not have observed those results yet.
    pub(crate) fn note_memory_rules_injected(&mut self, iteration: usize) {
        self.memory_rules_injected_iteration
            .get_or_insert(iteration);
    }

    pub(crate) fn memory_rules_injected_iteration(&self) -> Option<usize> {
        self.memory_rules_injected_iteration
    }

    pub(crate) fn memory_rules_visible(&self, iteration: usize) -> bool {
        self.memory_rules_injected_iteration
            .is_some_and(|injected| iteration > injected)
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
        assert!(!snapshot.memory_rules_visible(0));
    }

    #[test]
    fn memory_rules_only_become_visible_after_the_injecting_iteration() {
        let mut snapshot = TurnSnapshot::capture(
            "gui:chat".into(),
            "model".into(),
            false,
            None,
            false,
            false,
            "trace".into(),
        );
        snapshot.note_memory_rules_injected(2);
        snapshot.note_memory_rules_injected(3);
        assert_eq!(snapshot.memory_rules_injected_iteration(), Some(2));
        assert!(!snapshot.memory_rules_visible(2));
        assert!(snapshot.memory_rules_visible(3));
    }
}
