//! `memory_list` tool: list the applied memory projection.

use std::path::PathBuf;
use std::sync::Arc;

use agent_diva_core::memory::{MemoryCrudContext, MemoryListRequest, MemoryProvider};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

/// List tool for applied memory projection.
pub struct MemoryListTool {
    provider: Option<Arc<dyn MemoryProvider>>,
    workspace: Option<PathBuf>,
}

impl MemoryListTool {
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

impl Default for MemoryListTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MemoryListTool {
    fn name(&self) -> &str {
        "memory_list"
    }

    fn description(&self) -> &str {
        "List the applied memory authority. Returns visible entries with trust and provenance. Use before memory_update or memory_remove to discover the target record ids."
    }

    fn parameters(&self) -> Value {
        json!({"type": "object", "properties": {"limit": {"type": "integer", "description": "Maximum number of entries to return"}}, "required": []})
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let request: MemoryListRequest = serde_json::from_value(args)
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
            .memory_list(
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
        let tool = MemoryListTool::new();
        let result = tool.execute(valid_args()).await.unwrap();
        assert!(result.contains("\"status\":\"failed\""));
    }

    #[tokio::test]
    async fn invalid_args_is_error() {
        let tool = MemoryListTool::new();
        assert!(tool.execute(json!(42)).await.is_err());
    }
}
