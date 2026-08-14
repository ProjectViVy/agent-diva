//! Bounded read-only `actmem` tool.

use std::sync::Arc;

use agent_diva_core::memory::{ActmemReadRequest, MemoryProvider};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct ActmemTool {
    provider: Option<Arc<dyn MemoryProvider>>,
}

impl ActmemTool {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: Arc<dyn MemoryProvider>) -> Self {
        Self {
            provider: Some(provider),
        }
    }
}

impl Default for ActmemTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ActmemTool {
    fn name(&self) -> &str {
        "actmem"
    }

    fn description(&self) -> &str {
        "Read one bounded ACTMEM projection: pulse, recap, work, head, capsule directory, or one named capsule."
    }

    fn parameters(&self) -> Value {
        json!({
            "type":"object",
            "properties":{
                "target":{"type":"string","enum":["pulse","recap","work","head","capsules","capsule"]},
                "capsule_name":{"type":"string"}
            },
            "required":["target"]
        })
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let request: ActmemReadRequest = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("memory provider unavailable".to_string()))?;
        let response = provider
            .actmem_read(request)
            .await
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        serde_json::to_string(&response)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}
