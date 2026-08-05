//! `ask_user` tool: structured conversational clarify with a blocking wait.

use agent_diva_core::ask_user::{AskUserCoordinator, AskUserError};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Maximum number of choices accepted by the tool schema.
pub const MAX_CHOICES: usize = 4;

/// Tool for asking the user a structured question and waiting for an answer.
///
/// Without a coordinator the tool is registered but reports `unavailable`,
/// so headless runtimes degrade gracefully instead of hanging a turn.
pub struct AskUserTool {
    coordinator: Option<AskUserCoordinator>,
}

impl AskUserTool {
    /// Create a tool that reports `unavailable` (no interactive surface).
    pub fn new() -> Self {
        Self { coordinator: None }
    }

    /// Create a tool backed by a coordinator shared with the surface layer.
    pub fn with_coordinator(coordinator: AskUserCoordinator) -> Self {
        Self {
            coordinator: Some(coordinator),
        }
    }
}

impl Default for AskUserTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for AskUserTool {
    fn name(&self) -> &str {
        "ask_user"
    }

    fn description(&self) -> &str {
        "Ask the user a structured question and wait for their answer. Use this for preference research, ambiguity resolution, or trade-off decisions when the task needs user input to continue. The tool suspends until the user answers, cancels, or the question times out."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "question": {
                    "type": "string",
                    "description": "The question to ask the user"
                },
                "choices": {
                    "type": "array",
                    "items": {"type": "string"},
                    "maxItems": 4,
                    "description": "Optional: up to 4 predefined choices; the user can still give a free-text answer when allow_other is set"
                },
                "allow_other": {
                    "type": "boolean",
                    "description": "Optional: allow the user to provide a free-text answer (default false)"
                },
                "context": {
                    "type": "string",
                    "description": "Optional: extra context shown with the question"
                }
            },
            "required": ["question"]
        })
    }

    /// Override the registry's global timeout so a pending question can wait
    /// for the coordinator's full lifetime (default 10 minutes).
    fn timeout_secs(&self) -> Option<u64> {
        self.coordinator
            .as_ref()
            .map(|coordinator| coordinator.timeout_secs())
    }

    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let Some(coordinator) = &self.coordinator else {
            return Ok(json!({"status": "unavailable"}).to_string());
        };

        let question = params
            .get("question")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'question' parameter".to_string()))?
            .to_string();

        let choices = params
            .get("choices")
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if choices.len() > MAX_CHOICES {
            return Err(ToolError::InvalidParams(format!(
                "'choices' must have at most {MAX_CHOICES} items"
            )));
        }

        let allow_other = params
            .get("allow_other")
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        let context = params
            .get("context")
            .and_then(|value| value.as_str())
            .map(str::to_string);

        match coordinator
            .request(question, choices, allow_other, context)
            .await
        {
            Ok(response) => serde_json::to_string(&response).map_err(|error| {
                ToolError::ExecutionFailed(format!(
                    "failed to serialize ask_user response: {error}"
                ))
            }),
            Err(AskUserError::Expired) => Ok(json!({"status": "expired"}).to_string()),
            Err(_) => Ok(json!({"status": "unavailable"}).to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn unavailable_without_coordinator() {
        let tool = AskUserTool::new();
        let result = tool.execute(json!({"question": "Pick?"})).await.unwrap();
        assert!(result.contains("\"unavailable\""));
    }

    #[tokio::test]
    async fn missing_question_is_invalid_params() {
        let tool = AskUserTool::with_coordinator(AskUserCoordinator::default());
        let result = tool.execute(json!({})).await;
        assert!(matches!(result, Err(ToolError::InvalidParams(_))));
    }

    #[tokio::test]
    async fn too_many_choices_rejected() {
        let tool = AskUserTool::with_coordinator(AskUserCoordinator::default());
        let result = tool
            .execute(json!({"question": "Pick?", "choices": ["a", "b", "c", "d", "e"]}))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidParams(_))));
    }

    #[tokio::test]
    async fn answer_flows_back_as_tool_result() {
        let coordinator = AskUserCoordinator::new(Duration::from_secs(30));
        let tool = AskUserTool::with_coordinator(coordinator.clone());
        let execute = tokio::spawn(async move {
            tool.execute(json!({
                "question": "Which option?",
                "choices": ["A", "B"],
                "allow_other": true
            }))
            .await
        });
        let question_id = loop {
            if let Some(question) = coordinator.pending().await.into_iter().next() {
                break question.question_id;
            }
            tokio::task::yield_now().await;
        };
        coordinator
            .answer(&question_id, Some(0), None)
            .await
            .unwrap();
        let result = execute.await.unwrap().unwrap();
        let value: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(value["status"], "answered");
        assert_eq!(value["selected"], "A");
        assert_eq!(value["selected_index"], 0);
    }

    #[tokio::test]
    async fn timeout_secs_matches_coordinator() {
        let tool = AskUserTool::with_coordinator(AskUserCoordinator::new(Duration::from_secs(600)));
        assert_eq!(tool.timeout_secs(), Some(600));
        assert_eq!(AskUserTool::new().timeout_secs(), None);
    }
}
