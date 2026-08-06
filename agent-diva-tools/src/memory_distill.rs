//! `memory_distill` tool: proactive experience distillation into a skill.

use std::path::PathBuf;
use std::sync::Arc;

use agent_diva_core::memory::{MemoryCrudContext, MemoryDistillRequest, MemoryProvider};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Distill tool that writes task experience as a skill.
pub struct MemoryDistillTool {
    provider: Option<Arc<dyn MemoryProvider>>,
    workspace: Option<PathBuf>,
}

impl MemoryDistillTool {
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

impl Default for MemoryDistillTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for MemoryDistillTool {
    fn name(&self) -> &str {
        "memory_distill"
    }

    fn description(&self) -> &str {
        "Distill action-verified experience from a completed task into a reusable skill (SKILL.md under workspace skills). A new skill is created immediately; overwriting an existing skill requires review. Only include information that was verified by action and is likely to be useful again."
    }

    fn parameters(&self) -> Value {
        json!({"type": "object", "properties": {"skill_name": {"type": "string", "description": "Skill name (kebab-case) for the distilled experience"}, "content": {"type": "string", "description": "Concise, action-verified experience content"}, "evidence": {"type": "string", "description": "Optional session-context evidence backing the distillation (for example key working-memory facts or the task outcome)"}}, "required": ["skill_name", "content"]})
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let request: MemoryDistillRequest = serde_json::from_value(args)
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
            .memory_distill(
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
        let tool = MemoryDistillTool::new();
        let result = tool.execute(valid_args()).await.unwrap();
        assert!(result.contains("\"status\":\"failed\""));
    }

    #[tokio::test]
    async fn invalid_args_is_error() {
        let tool = MemoryDistillTool::new();
        assert!(tool.execute(json!(42)).await.is_err());
    }
}
