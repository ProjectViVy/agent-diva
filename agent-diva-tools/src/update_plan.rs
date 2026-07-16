//! Lightweight TODO-list tool for normal chat.
//!
//! `update_plan` lets the model publish a per-turn checklist that is displayed
//! in the chat history. It does **not** execute the steps, nor does it write
//! to any persistent store or plan registry.

use agent_diva_core::planning::update_plan::{
    PlanItemStatus, UpdatePlanArgs, MAX_UPDATE_PLAN_EXPLANATION_LEN, MAX_UPDATE_PLAN_ITEMS,
    MAX_UPDATE_PLAN_STEP_LEN,
};
use agent_diva_tooling::{Result, Tool, ToolError};
use serde_json::Value;

/// `update_plan` tool — validates and returns a lightweight plan update.
///
/// This tool intentionally does **not** write to `JsonlTodoStore`,
/// `EphemeralPlanRegistry`, or any other store. The caller (e.g. the agent
/// handler) is responsible for emitting the corresponding event.
#[derive(Debug, Clone, Default)]
pub struct UpdatePlanTool;

impl UpdatePlanTool {
    pub fn new() -> Self {
        Self
    }
}

#[async_trait::async_trait]
impl Tool for UpdatePlanTool {
    fn name(&self) -> &str {
        "update_plan"
    }

    fn description(&self) -> &str {
        "Create or update the current task's lightweight TODO/progress checklist in normal chat. \
         This is not Plan mode and does not execute or persist the steps."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "explanation": {
                    "type": "string",
                    "description": "Optional explanation of the plan update",
                    "maxLength": 1000
                },
                "plan": {
                    "type": "array",
                    "description": "The TODO checklist items (max 20, at most one in_progress)",
                    "maxItems": 20,
                    "items": {
                        "type": "object",
                        "properties": {
                            "step": {
                                "type": "string",
                                "description": "The TODO step text",
                                "maxLength": 500
                            },
                            "status": {
                                "type": "string",
                                "enum": ["pending", "in_progress", "completed"]
                            }
                        },
                        "required": ["step", "status"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["plan"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let args: UpdatePlanArgs = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;

        if args.plan.is_empty() {
            return Err(ToolError::InvalidArguments(
                "plan must contain at least one item".to_string(),
            ));
        }
        if args.plan.len() > MAX_UPDATE_PLAN_ITEMS {
            return Err(ToolError::InvalidArguments(format!(
                "plan cannot exceed {} items",
                MAX_UPDATE_PLAN_ITEMS
            )));
        }
        if args
            .plan
            .iter()
            .filter(|item| item.status == PlanItemStatus::InProgress)
            .count()
            > 1
        {
            return Err(ToolError::InvalidArguments(
                "plan cannot contain more than one in_progress item".to_string(),
            ));
        }
        if let Some(explanation) = &args.explanation {
            if explanation.len() > MAX_UPDATE_PLAN_EXPLANATION_LEN {
                return Err(ToolError::InvalidArguments(format!(
                    "explanation cannot exceed {} characters",
                    MAX_UPDATE_PLAN_EXPLANATION_LEN
                )));
            }
        }
        for (index, item) in args.plan.iter().enumerate() {
            let step = item.step.trim();
            if step.is_empty() {
                return Err(ToolError::InvalidArguments(format!(
                    "plan item {} step cannot be empty",
                    index + 1
                )));
            }
            if item.step.len() > MAX_UPDATE_PLAN_STEP_LEN {
                return Err(ToolError::InvalidArguments(format!(
                    "plan item {} step cannot exceed {} characters",
                    index + 1,
                    MAX_UPDATE_PLAN_STEP_LEN
                )));
            }
        }

        Ok(format!(
            "TODO checklist updated with {} item(s).",
            args.plan.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn update_plan_valid() {
        let tool = UpdatePlanTool::new();
        let args = serde_json::json!({
            "explanation": "Steps to finish the feature",
            "plan": [
                {"step": "Design API", "status": "pending"},
                {"step": "Implement backend", "status": "in_progress"},
                {"step": "Write tests", "status": "completed"}
            ]
        });
        let result = tool.execute(args).await.unwrap();
        assert!(result.contains("TODO checklist updated with 3 item(s)"));
    }

    #[tokio::test]
    async fn update_plan_invalid_status() {
        let tool = UpdatePlanTool::new();
        let args = serde_json::json!({
            "plan": [{"step": "Bad step", "status": "Blocked"}]
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Invalid arguments")
                || err.contains("unknown variant")
                || err.contains("Blocked"),
            "expected invalid status error, got: {}",
            err
        );
    }

    #[tokio::test]
    async fn update_plan_empty_plan() {
        let tool = UpdatePlanTool::new();
        let args = serde_json::json!({"plan": []});
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("plan must contain at least one item"));
    }

    #[tokio::test]
    async fn update_plan_rejects_multiple_in_progress_items() {
        let tool = UpdatePlanTool::new();
        let result = tool
            .execute(serde_json::json!({
                "plan": [
                    {"step": "First", "status": "in_progress"},
                    {"step": "Second", "status": "in_progress"}
                ]
            }))
            .await;

        assert!(result
            .unwrap_err()
            .to_string()
            .contains("more than one in_progress"));
    }

    #[tokio::test]
    async fn update_plan_too_long() {
        let tool = UpdatePlanTool::new();
        let plan: Vec<Value> = (0..MAX_UPDATE_PLAN_ITEMS + 1)
            .map(|i| {
                serde_json::json!({
                    "step": format!("step {}", i),
                    "status": "pending"
                })
            })
            .collect();
        let args = serde_json::json!({"plan": plan});
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("plan cannot exceed 20 items"));
    }

    #[tokio::test]
    async fn update_plan_overlong_step() {
        let tool = UpdatePlanTool::new();
        let args = serde_json::json!({
            "plan": [{"step": "x".repeat(MAX_UPDATE_PLAN_STEP_LEN + 1), "status": "pending"}]
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("step cannot exceed 500 characters"));
    }

    #[tokio::test]
    async fn update_plan_overlong_explanation() {
        let tool = UpdatePlanTool::new();
        let args = serde_json::json!({
            "explanation": "x".repeat(MAX_UPDATE_PLAN_EXPLANATION_LEN + 1),
            "plan": [{"step": "A valid step", "status": "pending"}]
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("explanation cannot exceed 1000 characters"));
    }

    #[tokio::test]
    async fn update_plan_empty_step() {
        let tool = UpdatePlanTool::new();
        let args = serde_json::json!({
            "plan": [{"step": "   ", "status": "pending"}]
        });
        let result = tool.execute(args).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("step cannot be empty"));
    }

    #[tokio::test]
    async fn update_plan_statuses_match_schema() {
        // Verify that the serde representation matches the schema enum names.
        assert_eq!(
            serde_json::to_string(&PlanItemStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&PlanItemStatus::InProgress).unwrap(),
            "\"in_progress\""
        );
        assert_eq!(
            serde_json::to_string(&PlanItemStatus::Completed).unwrap(),
            "\"completed\""
        );
    }
}
