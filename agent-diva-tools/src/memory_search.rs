//! `memory_search` tool: search applied memory.

use std::path::PathBuf;
use std::sync::Arc;

use agent_diva_core::memory::{MemoryCrudContext, MemoryProvider, MemorySearchRequest};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Search tool for applied memory.
pub struct MemorySearchTool {
    provider: Option<Arc<dyn MemoryProvider>>,
    workspace: Option<PathBuf>,
}

impl MemorySearchTool {
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

impl Default for MemorySearchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MemorySearchTool {
    fn name(&self) -> &str {
        "memory_search"
    }

    fn description(&self) -> &str {
        "Search the applied memory authority for entries matching a query. Use when the user asks whether something was remembered, or when recall from memory is needed mid-conversation."
    }

    fn parameters(&self) -> Value {
        json!({"type": "object", "properties": {"query": {"type": "string", "description": "Search query"}}, "required": ["query"]})
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let request: MemorySearchRequest = serde_json::from_value(args)
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
            .memory_search(
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
        let tool = MemorySearchTool::new();
        let result = tool.execute(valid_args()).await.unwrap();
        assert!(result.contains("\"status\":\"failed\""));
    }

    #[tokio::test]
    async fn invalid_args_is_error() {
        let tool = MemorySearchTool::new();
        assert!(tool.execute(json!(42)).await.is_err());
    }
}
