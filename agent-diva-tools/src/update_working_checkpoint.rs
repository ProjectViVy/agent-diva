//! `session_checkpoint` tool: session-scoped volatile checkpoint.

use std::path::PathBuf;
use std::sync::Arc;

use agent_diva_core::memory::{MemoryProvider, SessionCheckpointWriteRequest};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

/// Session working checkpoint tool (volatile, non-authoritative).
///
/// The checkpoint is session-scoped: it is injected into live-turn context as
/// a session checkpoint and cleared on session end. It never becomes long-term
/// authority; promote durable knowledge with `memory_distill`.
pub struct SessionCheckpointTool {
    provider: Option<Arc<dyn MemoryProvider>>,
    workspace: Option<PathBuf>,
    session_id: Option<String>,
}

impl SessionCheckpointTool {
    /// Create a tool that reports the provider as unavailable.
    pub fn new() -> Self {
        Self {
            provider: None,
            workspace: None,
            session_id: None,
        }
    }

    /// Create a tool backed by the configured memory provider.
    pub fn with_provider(provider: Arc<dyn MemoryProvider>, workspace: PathBuf) -> Self {
        Self {
            provider: Some(provider),
            workspace: Some(workspace),
            session_id: None,
        }
    }

    /// Bind the active session key so checkpoints land in the right session.
    pub fn with_session(mut self, session_id: Option<String>) -> Self {
        self.session_id = session_id;
        self
    }
}

impl Default for SessionCheckpointTool {
    fn default() -> Self {
        Self::new()
    }
}

/// User-visible checkpoint arguments; workspace and session are injected by
/// the tool assembly, never requested from the model.
#[derive(serde::Deserialize)]
struct CheckpointArgs {
    key_info: String,
    #[serde(default)]
    related_sops: Vec<String>,
    #[serde(default)]
    content: String,
}

#[async_trait]
impl Tool for SessionCheckpointTool {
    fn name(&self) -> &str {
        "session_checkpoint"
    }

    fn description(&self) -> &str {
        "Record the current task state in this session's volatile checkpoint. It is visible only in this session and is physically cleared on reset, delete, or explicit session end; it is not long-term Memory."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "key_info": {
                    "type": "string",
                    "description": "Structured key facts of the current task state"
                },
                "related_sops": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Related skill/SOP names"
                },
                "content": {
                    "type": "string",
                    "description": "Free-form checkpoint content"
                }
            },
            "required": ["key_info"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let args: CheckpointArgs = serde_json::from_value(args)
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
        let Some(session_id) = self.session_id.clone() else {
            return Ok(json!({"status": "failed", "reason": "no active session"}).to_string());
        };
        let outcome = provider
            .session_checkpoint_write(SessionCheckpointWriteRequest {
                workspace_root: workspace,
                session_id,
                key_info: args.key_info,
                related_sops: args.related_sops,
                content: args.content,
            })
            .await
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        serde_json::to_string(&outcome)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::memory::{
        PrefetchRequest, PrefetchResponse, PrefetchStatus, SessionEndRequest, SessionEndResponse,
        SessionEndStatus, SyncTurnRequest, SyncTurnResponse, SyncTurnStatus, SystemPromptBlock,
        SystemPromptRequest, SystemPromptResponse,
    };

    struct DummyProvider;

    #[async_trait]
    impl MemoryProvider for DummyProvider {
        fn system_prompt_block(
            &self,
            _request: &SystemPromptRequest,
        ) -> agent_diva_core::Result<SystemPromptResponse> {
            Ok(SystemPromptResponse::ready(SystemPromptBlock {
                shape: agent_diva_core::memory::StartupInjectionShape::CompactRenderedMarkdown,
                markdown: String::new(),
            }))
        }

        async fn prefetch(
            &self,
            _request: PrefetchRequest,
        ) -> agent_diva_core::Result<PrefetchResponse> {
            Ok(PrefetchResponse {
                status: PrefetchStatus::SkippedNoIntent,
                prompt_block: None,
            })
        }

        async fn sync_turn(
            &self,
            _request: SyncTurnRequest,
        ) -> agent_diva_core::Result<SyncTurnResponse> {
            Ok(SyncTurnResponse {
                status: SyncTurnStatus::Noop,
            })
        }

        async fn on_session_end(
            &self,
            _request: SessionEndRequest,
        ) -> agent_diva_core::Result<SessionEndResponse> {
            Ok(SessionEndResponse {
                status: SessionEndStatus::Noop,
            })
        }
    }

    fn valid_args() -> serde_json::Value {
        serde_json::json!({"key_info": "probe", "content": "state"})
    }

    #[tokio::test]
    async fn without_provider_reports_failed() {
        let tool = SessionCheckpointTool::new();
        let result = tool.execute(valid_args()).await.unwrap();
        assert!(result.contains("\"status\":\"failed\""));
    }

    #[tokio::test]
    async fn without_session_reports_failed() {
        let tool =
            SessionCheckpointTool::with_provider(Arc::new(DummyProvider), PathBuf::from("/tmp/ws"));
        let result = tool.execute(valid_args()).await.unwrap();
        assert!(result.contains("no active session"));
    }

    #[tokio::test]
    async fn invalid_args_is_error() {
        let tool = SessionCheckpointTool::new();
        assert!(tool.execute(json!(42)).await.is_err());
    }
}
