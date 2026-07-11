use agent_diva_core::planning::{
    EphemeralPlanRegistry, ExecutionTodo, ExecutionTodoPriority, ExecutionTodoStatus,
};
use agent_diva_tooling::{Result, Tool, ToolError};
use chrono::Utc;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Clone)]
pub struct ExecutionTodoShowTool {
    store: Arc<EphemeralPlanRegistry>,
    execution_session_id: String,
}

impl ExecutionTodoShowTool {
    pub fn new(store: Arc<EphemeralPlanRegistry>, execution_session_id: String) -> Self {
        Self {
            store,
            execution_session_id,
        }
    }
}

#[async_trait::async_trait]
impl Tool for ExecutionTodoShowTool {
    fn name(&self) -> &str {
        "todo_show"
    }

    fn description(&self) -> &str {
        "Show the TODO list for the approved execution session."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "additionalProperties": false
        })
    }

    async fn execute(&self, _args: serde_json::Value) -> Result<String> {
        let todos = self
            .store
            .execution_todos(&self.execution_session_id)
            .await
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        if todos.is_empty() {
            return Ok("No execution TODOs have been created.".to_string());
        }
        Ok(render_todos(&todos))
    }
}

#[derive(Clone)]
pub struct ExecutionTodoWriteTool {
    store: Arc<EphemeralPlanRegistry>,
    execution_session_id: String,
}

impl ExecutionTodoWriteTool {
    pub fn new(store: Arc<EphemeralPlanRegistry>, execution_session_id: String) -> Self {
        Self {
            store,
            execution_session_id,
        }
    }
}

#[derive(Debug, Deserialize)]
struct TodoWriteInput {
    items: Vec<TodoWriteItem>,
}

#[derive(Debug, Deserialize)]
struct TodoWriteItem {
    id: Option<String>,
    title: String,
    detail: Option<String>,
    status: Option<ExecutionTodoStatus>,
    priority: Option<ExecutionTodoPriority>,
    evidence_ref: Option<String>,
    block_reason: Option<String>,
}

#[async_trait::async_trait]
impl Tool for ExecutionTodoWriteTool {
    fn name(&self) -> &str {
        "todo_write"
    }

    fn description(&self) -> &str {
        "Create or replace the TODO list for the approved execution session. TODOs are separate from the approved PLAN report."
    }

    fn parameters(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "items": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "id": { "type": "string" },
                            "title": { "type": "string" },
                            "detail": { "type": "string" },
                            "status": { "type": "string", "enum": ["Pending", "InProgress", "Blocked", "Completed", "Canceled"] },
                            "priority": { "type": "string", "enum": ["Low", "Normal", "High"] },
                            "evidence_ref": { "type": "string" },
                            "block_reason": { "type": "string" }
                        },
                        "required": ["title"],
                        "additionalProperties": false
                    }
                }
            },
            "required": ["items"],
            "additionalProperties": false
        })
    }

    async fn execute(&self, args: serde_json::Value) -> Result<String> {
        let input: TodoWriteInput = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let mut todos = Vec::with_capacity(input.items.len());
        for (index, item) in input.items.into_iter().enumerate() {
            let title = item.title.trim();
            if title.is_empty() {
                return Err(ToolError::InvalidArguments(
                    "execution TODO title cannot be empty".to_string(),
                ));
            }
            let status = item.status.unwrap_or(ExecutionTodoStatus::Pending);
            let block_reason = item.block_reason.and_then(non_empty);
            if status == ExecutionTodoStatus::Blocked && block_reason.is_none() {
                return Err(ToolError::InvalidArguments(
                    "blocked execution TODO requires block_reason".to_string(),
                ));
            }
            todos.push(ExecutionTodo {
                id: item
                    .id
                    .filter(|id| !id.trim().is_empty())
                    .unwrap_or_else(|| format!("todo-{}", index + 1)),
                execution_session_id: self.execution_session_id.clone(),
                title: title.to_string(),
                detail: item.detail.and_then(non_empty),
                status,
                priority: item.priority.unwrap_or(ExecutionTodoPriority::Normal),
                evidence_ref: item.evidence_ref.and_then(non_empty),
                block_reason,
                updated_at: Utc::now(),
            });
        }
        self.store
            .replace_execution_todos(&self.execution_session_id, &todos)
            .await
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        Ok(render_todos(&todos))
    }
}

fn non_empty(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn render_todos(todos: &[ExecutionTodo]) -> String {
    let mut lines = Vec::with_capacity(todos.len() + 1);
    lines.push("# Execution TODOs".to_string());
    for todo in todos {
        lines.push(format!(
            "- [{:?}] {:?}: {}",
            todo.status, todo.priority, todo.title
        ));
        if let Some(detail) = &todo.detail {
            lines.push(format!("  - detail: {detail}"));
        }
        if let Some(evidence) = &todo.evidence_ref {
            lines.push(format!("  - evidence: {evidence}"));
        }
        if let Some(reason) = &todo.block_reason {
            lines.push(format!("  - blocked: {reason}"));
        }
    }
    lines.join("\n")
}
