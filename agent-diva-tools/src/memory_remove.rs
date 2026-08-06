//! `memory_remove` tool: high-risk memory removal via review.

use std::path::PathBuf;
use std::sync::Arc;

use agent_diva_core::memory::{MemoryCrudContext, MemoryProvider, MemoryRemoveRequest};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Remove tool that creates a deprecation proposal.
pub struct MemoryRemoveTool {
    provider: Option<Arc<dyn MemoryProvider>>,
    workspace: Option<PathBuf>,
}

impl MemoryRemoveTool {
    /// Create a tool that reports the provider as unavailable.
    pub fn new() -> Self {
        Self {
            provider: None,
            workspace: None,
        }
    }

    /// Create a tool backed by the configured memory provider.
    pub fn with_provider(provider: Arc<dyn MemoryProvider>, workspace: PathBuf) -> Self {
        Self {
            provider: Some(provider),
            workspace: Some(workspace),
        }
    }
}

impl Default for MemoryRemoveTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MemoryRemoveTool {
    fn name(&self) -> &str {
        "memory_remove"
    }

    fn description(&self) -> &str {
        "Remove an applied memory record (forget). This creates a reviewable deprecation proposal; the record is tombstoned after approval so it stops appearing in prompts. Report the proposal id to the user."
    }

    fn parameters(&self) -> Value {
        json!({"type": "object", "properties": {"record_id": {"type": "string", "description": "Target record id from memory_list or memory_search"}, "reason": {"type": "string", "description": "Why the record should be forgotten"}}, "required": ["record_id", "reason"]})
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let request: MemoryRemoveRequest = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let Some(provider) = &self.provider else {
            return Ok(
                json!({"status": "failed", "reason": "memory provider unavailable"}).to_string(),
            );
        };
        let Some(workspace) = self.workspace.clone() else {
            return Ok(
                json!({"status": "failed", "reason": "memory workspace unavailable"}).to_string(),
            );
        };
        let outcome = provider
            .memory_remove(
                &MemoryCrudContext {
                    workspace_root: workspace,
                },
                request,
            )
            .await
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        serde_json::to_string(&outcome)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_args() -> serde_json::Value {
        // Tool-agnostic: unknown keys are ignored by serde, so a probe value
        // parses for every tool while still exercising the parse path.
        serde_json::json!({"content": "probe", "record_id": "r1", "query": "q", "skill_name": "s", "reason": "probe"})
    }

    #[tokio::test]
    async fn without_provider_reports_failed() {
        let tool = MemoryRemoveTool::new();
        let result = tool.execute(valid_args()).await.unwrap();
        assert!(result.contains("\"status\":\"failed\""));
    }

    #[tokio::test]
    async fn invalid_args_is_error() {
        let tool = MemoryRemoveTool::new();
        assert!(tool.execute(json!(42)).await.is_err());
    }
}
