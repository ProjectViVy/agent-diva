//! CAS edit tool for one registered ACTMEM Work subsection.

use std::sync::Arc;

use agent_diva_core::memory::{ActmemEditWorkRequest, MemoryProvider};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct ActmemEditWorkTool {
    provider: Option<Arc<dyn MemoryProvider>>,
}

impl ActmemEditWorkTool {
    pub fn new() -> Self {
        Self { provider: None }
    }

    pub fn with_provider(provider: Arc<dyn MemoryProvider>) -> Self {
        Self {
            provider: Some(provider),
        }
    }
}

impl Default for ActmemEditWorkTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ActmemEditWorkTool {
    fn name(&self) -> &str {
        "actmem_edit_work"
    }

    fn description(&self) -> &str {
        "Replace exactly one ACTMEM Work subsection (Goal, Open, Next, Constraints, or Pointers) using its base revision."
    }

    fn parameters(&self) -> Value {
        json!({"type":"object","properties":{
            "section":{"type":"string","enum":["Goal","Open","Next","Constraints","Pointers"]},
            "replacement":{"type":"string"},
            "base_revision":{"type":"integer","minimum":0}
        },"required":["section","replacement","base_revision"]})
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let request: ActmemEditWorkRequest = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("memory provider unavailable".to_string()))?;
        let response = provider
            .actmem_edit_work(request)
            .await
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        serde_json::to_string(&response)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}
