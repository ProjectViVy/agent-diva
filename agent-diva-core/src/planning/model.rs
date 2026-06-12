//! Domain model types for the planning subsystem.
//!
//! Contains all core structs and enums used to represent plans, steps,
//! todo items, and their lifecycle states.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::ids::{PlanId, TodoId};

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

/// Lifecycle phase of a plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanPhase {
    Explore,
    Plan,
    AwaitingApproval,
    Execute,
    Verify,
    Completed,
    Failed,
    Partial,
}

impl std::fmt::Display for PlanPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Explore => write!(f, "Explore"),
            Self::Plan => write!(f, "Plan"),
            Self::AwaitingApproval => write!(f, "AwaitingApproval"),
            Self::Execute => write!(f, "Execute"),
            Self::Verify => write!(f, "Verify"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Partial => write!(f, "Partial"),
        }
    }
}

/// Execution status shared by plans and steps.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Failed,
    Partial,
    Canceled,
}

impl std::fmt::Display for PlanStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::InProgress => write!(f, "InProgress"),
            Self::Blocked => write!(f, "Blocked"),
            Self::Completed => write!(f, "Completed"),
            Self::Failed => write!(f, "Failed"),
            Self::Partial => write!(f, "Partial"),
            Self::Canceled => write!(f, "Canceled"),
        }
    }
}

/// Status of an individual todo item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Blocked,
    Completed,
    Canceled,
}

impl std::fmt::Display for TodoStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pending => write!(f, "Pending"),
            Self::InProgress => write!(f, "InProgress"),
            Self::Blocked => write!(f, "Blocked"),
            Self::Completed => write!(f, "Completed"),
            Self::Canceled => write!(f, "Canceled"),
        }
    }
}

/// Priority level for todo items.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TodoPriority {
    Low,
    Normal,
    High,
}

impl std::fmt::Display for TodoPriority {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "Low"),
            Self::Normal => write!(f, "Normal"),
            Self::High => write!(f, "High"),
        }
    }
}

/// Outcome of a verification step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationVerdict {
    Pass,
    Fail,
    Partial,
}

impl std::fmt::Display for VerificationVerdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "Pass"),
            Self::Fail => write!(f, "Fail"),
            Self::Partial => write!(f, "Partial"),
        }
    }
}

// ---------------------------------------------------------------------------
// Structs
// ---------------------------------------------------------------------------

/// A high-level plan with metadata and lifecycle tracking.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Plan {
    pub id: PlanId,
    pub title: String,
    pub goal: String,
    pub phase: PlanPhase,
    pub status: PlanStatus,
    pub strategy: Option<String>,
    pub assumptions: Vec<String>,
    pub risks: Vec<String>,
    pub open_questions: Vec<String>,
    pub verification_verdict: Option<VerificationVerdict>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// An individual step within a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanStep {
    pub id: String,
    pub plan_id: PlanId,
    pub ordinal: i32,
    pub title: String,
    pub rationale: Option<String>,
    pub expected_output: Option<String>,
    pub status: PlanStatus,
    pub evidence_ref: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// A single actionable todo item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: TodoId,
    pub plan_step_id: Option<String>,
    pub title: String,
    pub detail: Option<String>,
    pub status: TodoStatus,
    pub priority: TodoPriority,
    pub evidence_ref: Option<String>,
    pub block_reason: Option<String>,
    pub updated_at: DateTime<Utc>,
}

/// A versioned collection of todo items for a plan.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TodoList {
    pub plan_id: PlanId,
    pub revision: i32,
    pub items: Vec<TodoItem>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plan_construction() {
        let now = Utc::now();
        let plan = Plan {
            id: PlanId::new(),
            title: "Test Plan".to_string(),
            goal: "Ship feature X".to_string(),
            phase: PlanPhase::Explore,
            status: PlanStatus::Pending,
            strategy: Some("Iterative approach".to_string()),
            assumptions: vec!["API is stable".to_string()],
            risks: vec!["Timeline slip".to_string()],
            open_questions: vec!["Need auth?".to_string()],
            verification_verdict: None,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(plan.title, "Test Plan");
        assert_eq!(plan.phase, PlanPhase::Explore);
        assert_eq!(plan.status, PlanStatus::Pending);
        assert!(plan.verification_verdict.is_none());
        assert_eq!(plan.assumptions.len(), 1);
        assert_eq!(plan.risks.len(), 1);
    }

    #[test]
    fn test_enum_display() {
        assert_eq!(PlanPhase::Explore.to_string(), "Explore");
        assert_eq!(PlanPhase::AwaitingApproval.to_string(), "AwaitingApproval");
        assert_eq!(PlanPhase::Partial.to_string(), "Partial");

        assert_eq!(PlanStatus::InProgress.to_string(), "InProgress");
        assert_eq!(PlanStatus::Canceled.to_string(), "Canceled");

        assert_eq!(TodoStatus::Pending.to_string(), "Pending");
        assert_eq!(TodoStatus::Blocked.to_string(), "Blocked");

        assert_eq!(TodoPriority::High.to_string(), "High");
        assert_eq!(TodoPriority::Low.to_string(), "Low");
        assert_eq!(TodoPriority::Normal.to_string(), "Normal");

        assert_eq!(VerificationVerdict::Pass.to_string(), "Pass");
        assert_eq!(VerificationVerdict::Fail.to_string(), "Fail");
        assert_eq!(VerificationVerdict::Partial.to_string(), "Partial");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let now = Utc::now();
        let plan = Plan {
            id: PlanId("test-plan-id".to_string()),
            title: "Roundtrip Test".to_string(),
            goal: "Verify serde".to_string(),
            phase: PlanPhase::Execute,
            status: PlanStatus::InProgress,
            strategy: None,
            assumptions: vec![],
            risks: vec![],
            open_questions: vec![],
            verification_verdict: Some(VerificationVerdict::Partial),
            created_at: now,
            updated_at: now,
        };

        let json = serde_json::to_string(&plan).unwrap();
        let back: Plan = serde_json::from_str(&json).unwrap();
        assert_eq!(plan, back);

        let todo = TodoItem {
            id: TodoId("todo-1".to_string()),
            plan_step_id: Some("step-1".to_string()),
            title: "Write tests".to_string(),
            detail: Some("Cover all paths".to_string()),
            status: TodoStatus::InProgress,
            priority: TodoPriority::High,
            evidence_ref: None,
            block_reason: None,
            updated_at: now,
        };

        let json = serde_json::to_string(&todo).unwrap();
        let back: TodoItem = serde_json::from_str(&json).unwrap();
        assert_eq!(todo, back);
    }

    #[test]
    fn test_plan_step_construction() {
        let now = Utc::now();
        let step = PlanStep {
            id: "step-1".to_string(),
            plan_id: PlanId("plan-1".to_string()),
            ordinal: 0,
            title: "Gather requirements".to_string(),
            rationale: Some("Understand scope".to_string()),
            expected_output: Some("Requirements doc".to_string()),
            status: PlanStatus::Pending,
            evidence_ref: None,
            created_at: now,
            updated_at: now,
        };
        assert_eq!(step.ordinal, 0);
        assert_eq!(step.status, PlanStatus::Pending);
    }

    #[test]
    fn test_todo_list_construction() {
        let now = Utc::now();
        let list = TodoList {
            plan_id: PlanId::new(),
            revision: 1,
            items: vec![],
            created_at: now,
            updated_at: now,
        };
        assert!(list.items.is_empty());
        assert_eq!(list.revision, 1);
    }
}
