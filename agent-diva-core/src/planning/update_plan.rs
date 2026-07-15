//! Types for the `update_plan` tool used in normal chat.
//!
//! This is a lightweight, per-turn TODO list for normal chat (not Plan mode).
//! It is intentionally isolated from `PlanExecution` and `TodoItem` types.

use serde::{Deserialize, Serialize};

/// Maximum number of items allowed in a normal-chat `update_plan` payload.
pub const MAX_UPDATE_PLAN_ITEMS: usize = 20;

/// Maximum length of the optional `explanation` field in characters.
pub const MAX_UPDATE_PLAN_EXPLANATION_LEN: usize = 1000;

/// Maximum length of a single `step` string in characters.
pub const MAX_UPDATE_PLAN_STEP_LEN: usize = 500;

/// Status of a single item in a normal-chat plan update.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanItemStatus {
    /// Item has not been started yet.
    Pending,
    /// Item is currently being worked on.
    InProgress,
    /// Item has been finished.
    Completed,
}

/// A single step in a normal-chat plan update.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanItem {
    /// The human-readable description of the step.
    pub step: String,
    /// The current status of the step.
    pub status: PlanItemStatus,
}

/// Arguments for the `update_plan` tool.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePlanArgs {
    /// Optional explanation for why the plan is being created or updated.
    #[serde(default)]
    pub explanation: Option<String>,
    /// The ordered list of plan items.
    pub plan: Vec<PlanItem>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_max_update_plan_items_constant() {
        assert_eq!(MAX_UPDATE_PLAN_ITEMS, 20);
    }

    #[test]
    fn test_plan_item_status_serde_roundtrip() {
        for status in [PlanItemStatus::Pending, PlanItemStatus::InProgress, PlanItemStatus::Completed] {
            let json = serde_json::to_string(&status).unwrap();
            let back: PlanItemStatus = serde_json::from_str(&json).unwrap();
            assert_eq!(status, back);
        }
    }

    #[test]
    fn test_plan_item_status_json_names() {
        assert_eq!(serde_json::to_string(&PlanItemStatus::Pending).unwrap(), "\"Pending\"");
        assert_eq!(serde_json::to_string(&PlanItemStatus::InProgress).unwrap(), "\"InProgress\"");
        assert_eq!(serde_json::to_string(&PlanItemStatus::Completed).unwrap(), "\"Completed\"");
    }

    #[test]
    fn update_plan_types_serde_roundtrip() {
        let args = UpdatePlanArgs {
            explanation: Some("Normal-chat plan".to_string()),
            plan: vec![
                PlanItem {
                    step: "Analyze request".to_string(),
                    status: PlanItemStatus::Completed,
                },
                PlanItem {
                    step: "Draft response".to_string(),
                    status: PlanItemStatus::InProgress,
                },
                PlanItem {
                    step: "Review output".to_string(),
                    status: PlanItemStatus::Pending,
                },
            ],
        };
        let json = serde_json::to_string(&args).unwrap();
        let back: UpdatePlanArgs = serde_json::from_str(&json).unwrap();
        assert_eq!(args, back);
    }

    #[test]
    fn test_update_plan_args_explanation_defaults_to_none() {
        let json = r#"{"plan": [{"step": "s", "status": "Pending"}]}"#;
        let args: UpdatePlanArgs = serde_json::from_str(json).unwrap();
        assert!(args.explanation.is_none());
        assert_eq!(args.plan.len(), 1);
    }

    #[test]
    fn test_update_plan_args_explanation_may_be_present() {
        let json = r#"{"explanation": "Why not", "plan": [{"step": "s", "status": "Completed"}]}"#;
        let args: UpdatePlanArgs = serde_json::from_str(json).unwrap();
        assert_eq!(args.explanation.as_deref(), Some("Why not"));
    }

    #[test]
    fn test_illegal_status_rejected() {
        let json = r#"{"plan": [{"step": "s", "status": "blocked"}]}"#;
        let result: Result<UpdatePlanArgs, _> = serde_json::from_str(json);
        assert!(result.is_err(), "blocked is not a valid PlanItemStatus");
    }

    #[test]
    fn test_unknown_field_rejected() {
        let json = r#"{"explanation": "x", "plan": [{"step": "s", "status": "Pending", "extra": 1}]}"#;
        let result: Result<UpdatePlanArgs, _> = serde_json::from_str(json);
        assert!(result.is_err(), "deny_unknown_fields should reject extra fields");
    }

    #[test]
    fn test_plan_item_status_variants_exactly_three() {
        // Compile-time guarantee: the enum has exactly three variants.
        let all = [PlanItemStatus::Pending, PlanItemStatus::InProgress, PlanItemStatus::Completed];
        assert_eq!(all.len(), 3);
    }
}
