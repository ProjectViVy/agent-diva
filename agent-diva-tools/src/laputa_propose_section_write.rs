//! `laputa_propose_section_write` tool: governed Frozen Core section write.

use std::path::PathBuf;
use std::sync::Arc;

use agent_diva_core::evolution::LaputaSectionName;
use agent_diva_core::memory::{MemoryCrudContext, MemoryProvider, SectionWriteProposalRequest};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Tool that creates a reviewable proposal writing a Frozen Core section.
pub struct LaputaProposeSectionWriteTool {
    provider: Option<Arc<dyn MemoryProvider>>,
    workspace: Option<PathBuf>,
}

impl LaputaProposeSectionWriteTool {
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

impl Default for LaputaProposeSectionWriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for LaputaProposeSectionWriteTool {
    fn name(&self) -> &str {
        "laputa_propose_section_write"
    }

    fn description(&self) -> &str {
        "Propose writing a Frozen Core persona section (identity, relationship, commitment, preferences). Touching persona authority is high-risk: this creates a reviewable governed proposal instead of writing directly. The section content must be valid JSON. Report the proposal id to the user."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "section": {
                    "type": "string",
                    "enum": ["identity", "relationship", "commitment", "preferences"],
                    "description": "Frozen Core section to write"
                },
                "content": {
                    "type": "string",
                    "description": "JSON content for the section file, e.g. {\"name\":\"diva\",\"voice\":\"...\"}"
                },
                "summary": {
                    "type": "string",
                    "description": "Optional one-line summary of what this write contains; used as proposal evidence"
                }
            },
            "required": ["section", "content"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let section = args
            .get("section")
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                ToolError::InvalidArguments("Missing 'section' parameter".to_string())
            })?;
        let section = section
            .parse::<LaputaSectionName>()
            .map_err(|_| ToolError::InvalidArguments(format!("Unknown section '{section}'")))?;
        if !matches!(
            section,
            LaputaSectionName::Identity
                | LaputaSectionName::Relationship
                | LaputaSectionName::Commitment
                | LaputaSectionName::Preferences
        ) {
            return Err(ToolError::InvalidArguments(format!(
                "Section '{section}' is not a Frozen Core persona section"
            )));
        }
        let content = args
            .get("content")
            .and_then(|value| value.as_str())
            .ok_or_else(|| ToolError::InvalidArguments("Missing 'content' parameter".to_string()))?
            .to_string();
        let summary = args
            .get("summary")
            .and_then(|value| value.as_str())
            .map(str::to_string);
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
        let request = SectionWriteProposalRequest {
            section,
            content,
            summary,
        };
        let outcome = provider
            .propose_section_write(
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

    #[tokio::test]
    async fn unavailable_without_provider() {
        let tool = LaputaProposeSectionWriteTool::new();
        let result = tool
            .execute(json!({
                "section": "identity",
                "content": "{\"name\":\"diva\"}"
            }))
            .await
            .unwrap();
        assert!(result.contains("\"failed\""));
    }

    #[tokio::test]
    async fn unknown_section_rejected() {
        let tool = LaputaProposeSectionWriteTool::new();
        let result = tool
            .execute(json!({
                "section": "memory_md",
                "content": "{}"
            }))
            .await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }

    #[tokio::test]
    async fn missing_parameters_rejected() {
        let tool = LaputaProposeSectionWriteTool::new();
        let result = tool.execute(json!({})).await;
        assert!(matches!(result, Err(ToolError::InvalidArguments(_))));
    }
}
