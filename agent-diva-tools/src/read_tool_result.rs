use agent_diva_core::tool_artifact::{
    ToolArtifactSecurityContext, ToolArtifactStore, MAX_READ_CHARS,
};
use agent_diva_tooling::{Result, Tool, ToolError};
use async_trait::async_trait;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

pub struct ReadToolResultTool {
    store: Arc<ToolArtifactStore>,
    context: ToolArtifactSecurityContext,
}

impl ReadToolResultTool {
    pub fn new(store: Arc<ToolArtifactStore>, context: ToolArtifactSecurityContext) -> Self {
        Self { store, context }
    }
}

#[derive(Deserialize)]
struct ReadArgs {
    artifact_id: String,
    start: Option<usize>,
    end: Option<usize>,
}

#[async_trait]
impl Tool for ReadToolResultTool {
    fn name(&self) -> &str {
        "read_tool_result"
    }

    fn description(&self) -> &str {
        "Read a session-bound tool-result artifact by opaque ID, optionally using a bounded [start,end) character range."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "artifact_id": {"type": "string"},
                "start": {"type": "integer", "minimum": 0},
                "end": {"type": "integer", "minimum": 0}
            },
            "required": ["artifact_id"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String> {
        let args: ReadArgs = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidParams(error.to_string()))?;
        let store = Arc::clone(&self.store);
        let context = self.context.clone();
        tokio::task::spawn_blocking(move || {
            store
                .read_range(&context, &args.artifact_id, args.start, args.end)
                .map(|read| {
                    json!({
                        "artifact_id": args.artifact_id,
                        "start": read.start,
                        "end": read.end,
                        "total_chars": read.total_chars,
                        "sha256": read.sha256,
                        "content": read.content,
                        "max_read_chars": MAX_READ_CHARS
                    })
                    .to_string()
                })
                .map_err(|error| ToolError::Error(error.code().to_string()))
        })
        .await
        .map_err(|error| ToolError::Error(format!("artifact_io: {error}")))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reads_bound_range_and_rejects_invalid_range() {
        let workspace = tempfile::tempdir().unwrap();
        let context = ToolArtifactSecurityContext::new(workspace.path(), "session");
        let store = Arc::new(ToolArtifactStore::new(workspace.path()));
        let metadata = store
            .put(&context, "exec", "call", "ok", "甲乙丙丁")
            .unwrap();
        let tool = ReadToolResultTool::new(store, context);
        let output = tool
            .execute(json!({"artifact_id": metadata.artifact_id, "start": 1, "end": 3}))
            .await
            .unwrap();
        assert!(output.contains("乙丙"));
        let error = tool
            .execute(json!({"artifact_id": metadata.artifact_id, "start": 3, "end": 2}))
            .await
            .unwrap_err();
        assert!(error.to_string().contains("artifact_range_invalid"));
    }
}
