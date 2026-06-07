//! Event types for the planning event log.
//!
//! Events are append-only and capture every meaningful state change in a
//! plan's lifecycle. They are stored as JSON payloads in the database.

use serde::{Deserialize, Serialize};

use super::ids::{PlanId, TodoId};
use super::model::{PlanPhase, PlanStatus, VerificationVerdict};

/// Events emitted during plan lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlanEvent {
    Created {
        plan_id: PlanId,
        title: String,
        goal: String,
    },
    Drafted {
        plan_id: PlanId,
    },
    PhaseTransition {
        plan_id: PlanId,
        from: PlanPhase,
        to: PlanPhase,
    },
    StatusChanged {
        plan_id: PlanId,
        from: PlanStatus,
        to: PlanStatus,
    },
    VerificationRecorded {
        plan_id: PlanId,
        verdict: VerificationVerdict,
    },
    Completed {
        plan_id: PlanId,
    },
    Failed {
        plan_id: PlanId,
        reason: String,
    },
    Partial {
        plan_id: PlanId,
    },
}

/// Events emitted during todo lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TodoEvent {
    Generated {
        plan_id: PlanId,
        count: usize,
    },
    Revised {
        plan_id: PlanId,
        revision: i32,
    },
    Started {
        todo_id: TodoId,
    },
    Completed {
        todo_id: TodoId,
        evidence_ref: Option<String>,
    },
    Blocked {
        todo_id: TodoId,
        reason: String,
    },
    Unblocked {
        todo_id: TodoId,
    },
    Canceled {
        todo_id: TodoId,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_event_serde_roundtrip() {
        let event = PlanEvent::Created {
            plan_id: PlanId("p1".to_string()),
            title: "Test".to_string(),
            goal: "Goal".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: PlanEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_todo_event_serde_roundtrip() {
        let event = TodoEvent::Blocked {
            todo_id: TodoId("t1".to_string()),
            reason: "Waiting on API".to_string(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: TodoEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(event, back);
    }

    #[test]
    fn test_all_plan_event_variants() {
        let pid = PlanId("p1".to_string());
        let events = vec![
            PlanEvent::Created { plan_id: pid.clone(), title: "T".to_string(), goal: "G".to_string() },
            PlanEvent::Drafted { plan_id: pid.clone() },
            PlanEvent::PhaseTransition { plan_id: pid.clone(), from: PlanPhase::Explore, to: PlanPhase::Plan },
            PlanEvent::StatusChanged { plan_id: pid.clone(), from: PlanStatus::Pending, to: PlanStatus::InProgress },
            PlanEvent::VerificationRecorded { plan_id: pid.clone(), verdict: VerificationVerdict::Pass },
            PlanEvent::Completed { plan_id: pid.clone() },
            PlanEvent::Failed { plan_id: pid.clone(), reason: "oops".to_string() },
            PlanEvent::Partial { plan_id: pid.clone() },
        ];
        for event in &events {
            let json = serde_json::to_string(event).unwrap();
            let back: PlanEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(*event, back);
        }
    }
}
