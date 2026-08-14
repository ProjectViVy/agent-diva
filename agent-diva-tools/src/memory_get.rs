//! `memory_get` tool: retrieve one BML record by id.

use std::{path::PathBuf, sync::Arc};

use agent_diva_core::memory::{MemoryCrudContext, MemoryGetRequest, MemoryProvider};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct MemoryGetTool {
    provider: Option<Arc<dyn MemoryProvider>>,
    workspace: Option<PathBuf>,
}

impl MemoryGetTool {
    pub fn new() -> Self {
        Self {
            provider: None,
            workspace: None,
        }
    }

    pub fn with_provider(provider: Arc<dyn MemoryProvider>, workspace: PathBuf) -> Self {
        Self {
            provider: Some(provider),
            workspace: Some(workspace),
        }
    }
}

impl Default for MemoryGetTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MemoryGetTool {
    fn name(&self) -> &str {
        "memory_get"
    }

    fn description(&self) -> &str {
        "Retrieve one visible long-term Memory record by stable id, including its current revision."
    }

    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{"record_id":{"type":"string"}},"required":["record_id"]})
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let request: MemoryGetRequest = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("memory provider unavailable".to_string()))?;
        let workspace = self.workspace.clone().ok_or_else(|| {
            ToolError::ExecutionFailed("memory workspace unavailable".to_string())
        })?;
        let outcome = provider
            .memory_get(
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
