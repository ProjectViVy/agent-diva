//! Pure lifecycle and capability policy for the planning subsystem.
//!
//! This module projects persisted [`PlanPhase`] values into runtime mode
//! states and centralizes the fail-closed capability matrix used by later
//! approval and agent-loop waves.

use thiserror::Error;

use super::model::PlanPhase;

/// Runtime planning state derived from a persisted [`PlanPhase`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanModeState {
    Exploring,
    Drafting,
    AwaitingApproval,
    Executing,
    Verifying,
    Closed,
}

impl From<&PlanPhase> for PlanModeState {
    fn from(phase: &PlanPhase) -> Self {
        match phase {
            PlanPhase::Explore => Self::Exploring,
            PlanPhase::Plan => Self::Drafting,
            PlanPhase::AwaitingApproval => Self::AwaitingApproval,
            PlanPhase::Execute => Self::Executing,
            PlanPhase::Verify => Self::Verifying,
            PlanPhase::Completed | PlanPhase::Failed | PlanPhase::Partial => Self::Closed,
        }
    }
}

/// Evaluates a capability directly from the persisted phase.
pub fn allows_for_phase(phase: &PlanPhase, capability: ToolCapability) -> bool {
    allows(PlanModeState::from(phase), capability)
}

/// A normalized operation category used for planning runtime authorization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCapability {
    Inspect,
    PlanningRecord,
    WorkItem,
    WorkspaceWrite,
    Execute,
    External,
    /// Used when a tool cannot be mapped to a recognized capability.
    Unknown,
}

/// Lifecycle-policy validation failures.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum PlanningPolicyError {
    #[error("invalid plan transition: {from} -> {to}")]
    InvalidTransition { from: PlanPhase, to: PlanPhase },
}

/// Returns whether a state may use a capability.
///
/// The matrix is intentionally fail-closed. In particular, `Unknown` is
/// denied in every state so callers can safely map unrecognized tools to it.
pub fn allows(state: PlanModeState, capability: ToolCapability) -> bool {
    matches!(
        (state, capability),
        (
            PlanModeState::Exploring | PlanModeState::Drafting,
            ToolCapability::Inspect | ToolCapability::PlanningRecord,
        ) | (PlanModeState::AwaitingApproval, ToolCapability::Inspect)
            | (
                PlanModeState::Executing,
                ToolCapability::Inspect
                    | ToolCapability::PlanningRecord
                    | ToolCapability::WorkItem
                    | ToolCapability::WorkspaceWrite
                    | ToolCapability::Execute
                    | ToolCapability::External,
            )
            | (
                PlanModeState::Verifying,
                ToolCapability::Inspect | ToolCapability::PlanningRecord | ToolCapability::Execute,
            )
    )
}

/// Returns whether the lifecycle edge from `from` to `to` is valid.
pub fn is_valid_transition(from: &PlanPhase, to: &PlanPhase) -> bool {
    if *to == PlanPhase::Failed
        && !matches!(
            from,
            PlanPhase::Completed | PlanPhase::Failed | PlanPhase::Partial
        )
    {
        return true;
    }

    matches!(
        (from, to),
        (PlanPhase::Explore, PlanPhase::Plan)
            | (PlanPhase::Plan, PlanPhase::AwaitingApproval)
            | (PlanPhase::AwaitingApproval, PlanPhase::Execute)
            | (PlanPhase::Execute, PlanPhase::Verify)
            | (PlanPhase::Execute, PlanPhase::Completed)
            | (PlanPhase::Verify, PlanPhase::Completed)
            | (PlanPhase::Verify, PlanPhase::Partial)
    )
}

/// Validates a lifecycle edge without mutating persisted state.
pub fn validate_transition(from: &PlanPhase, to: &PlanPhase) -> Result<(), PlanningPolicyError> {
    if is_valid_transition(from, to) {
        Ok(())
    } else {
        Err(PlanningPolicyError::InvalidTransition {
            from: from.clone(),
            to: to.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CAPABILITIES: [ToolCapability; 7] = [
        ToolCapability::Inspect,
        ToolCapability::PlanningRecord,
        ToolCapability::WorkItem,
        ToolCapability::WorkspaceWrite,
        ToolCapability::Execute,
        ToolCapability::External,
        ToolCapability::Unknown,
    ];

    #[test]
    fn phase_projection_preserves_lifecycle_meaning() {
        assert_eq!(
            PlanModeState::from(&PlanPhase::Explore),
            PlanModeState::Exploring
        );
        assert_eq!(
            PlanModeState::from(&PlanPhase::Plan),
            PlanModeState::Drafting
        );
        assert_eq!(
            PlanModeState::from(&PlanPhase::AwaitingApproval),
            PlanModeState::AwaitingApproval
        );
        assert_eq!(
            PlanModeState::from(&PlanPhase::Execute),
            PlanModeState::Executing
        );
        assert_eq!(
            PlanModeState::from(&PlanPhase::Verify),
            PlanModeState::Verifying
        );

        for phase in [PlanPhase::Completed, PlanPhase::Failed, PlanPhase::Partial] {
            assert_eq!(PlanModeState::from(&phase), PlanModeState::Closed);
        }
    }

    #[test]
    fn capability_matrix_is_fail_closed() {
        let read_only_states = [PlanModeState::Exploring, PlanModeState::Drafting];

        for state in read_only_states {
            for capability in CAPABILITIES {
                let expected = matches!(
                    capability,
                    ToolCapability::Inspect | ToolCapability::PlanningRecord
                );
                assert_eq!(
                    allows(state, capability),
                    expected,
                    "{state:?} / {capability:?}"
                );
            }
        }

        for capability in CAPABILITIES {
            assert_eq!(
                allows(PlanModeState::AwaitingApproval, capability),
                capability == ToolCapability::Inspect,
                "AwaitingApproval / {capability:?}"
            );
        }

        for capability in CAPABILITIES {
            assert_eq!(
                allows(PlanModeState::Executing, capability),
                capability != ToolCapability::Unknown,
                "Executing / {capability:?}"
            );
            assert_eq!(
                allows(PlanModeState::Verifying, capability),
                matches!(
                    capability,
                    ToolCapability::Inspect
                        | ToolCapability::PlanningRecord
                        | ToolCapability::Execute
                ),
                "Verifying / {capability:?}"
            );
            assert!(!allows(PlanModeState::Closed, capability));
        }
    }

    #[test]
    fn valid_transitions_match_existing_lifecycle_contract() {
        let valid = [
            (PlanPhase::Explore, PlanPhase::Plan),
            (PlanPhase::Plan, PlanPhase::AwaitingApproval),
            (PlanPhase::AwaitingApproval, PlanPhase::Execute),
            (PlanPhase::Execute, PlanPhase::Verify),
            (PlanPhase::Execute, PlanPhase::Completed),
            (PlanPhase::Verify, PlanPhase::Completed),
            (PlanPhase::Verify, PlanPhase::Partial),
        ];

        for (from, to) in valid {
            assert!(is_valid_transition(&from, &to), "{from} -> {to}");
            assert_eq!(validate_transition(&from, &to), Ok(()));
        }

        for from in [
            PlanPhase::Explore,
            PlanPhase::Plan,
            PlanPhase::AwaitingApproval,
            PlanPhase::Execute,
            PlanPhase::Verify,
            PlanPhase::Completed,
            PlanPhase::Failed,
            PlanPhase::Partial,
        ] {
            assert_eq!(
                is_valid_transition(&from, &PlanPhase::Failed),
                !matches!(
                    from,
                    PlanPhase::Completed | PlanPhase::Failed | PlanPhase::Partial
                )
            );
        }
    }

    #[test]
    fn invalid_transition_returns_typed_error() {
        let result = validate_transition(&PlanPhase::Explore, &PlanPhase::Execute);

        assert_eq!(
            result,
            Err(PlanningPolicyError::InvalidTransition {
                from: PlanPhase::Explore,
                to: PlanPhase::Execute,
            })
        );
    }

    #[test]
    fn every_undocumented_transition_is_rejected() {
        let phases = [
            PlanPhase::Explore,
            PlanPhase::Plan,
            PlanPhase::AwaitingApproval,
            PlanPhase::Execute,
            PlanPhase::Verify,
            PlanPhase::Completed,
            PlanPhase::Failed,
            PlanPhase::Partial,
        ];
        for from in &phases {
            for to in &phases {
                let documented = matches!(
                    (from, to),
                    (PlanPhase::Explore, PlanPhase::Plan)
                        | (PlanPhase::Plan, PlanPhase::AwaitingApproval)
                        | (PlanPhase::AwaitingApproval, PlanPhase::Execute)
                        | (PlanPhase::Execute, PlanPhase::Verify)
                        | (PlanPhase::Execute, PlanPhase::Completed)
                        | (PlanPhase::Verify, PlanPhase::Completed)
                        | (PlanPhase::Verify, PlanPhase::Partial)
                ) || (*to == PlanPhase::Failed
                    && !matches!(
                        from,
                        PlanPhase::Completed | PlanPhase::Failed | PlanPhase::Partial
                    ));
                assert_eq!(is_valid_transition(from, to), documented, "{from} -> {to}");
            }
        }
    }
}
