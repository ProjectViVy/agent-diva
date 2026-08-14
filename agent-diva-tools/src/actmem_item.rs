//! CAS item tools for ACTMEM Work convergence.

use std::sync::Arc;

use agent_diva_core::memory::{ActmemItemRequest, MemoryProvider};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};

pub struct ActmemCompleteTool {
    provider: Option<Arc<dyn MemoryProvider>>,
}

pub struct ActmemDropTool {
    provider: Option<Arc<dyn MemoryProvider>>,
}

macro_rules! constructors {
    ($name:ident) => {
        impl $name {
            pub fn new() -> Self {
                Self { provider: None }
            }

            pub fn with_provider(provider: Arc<dyn MemoryProvider>) -> Self {
                Self {
                    provider: Some(provider),
                }
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }
    };
}

constructors!(ActmemCompleteTool);
constructors!(ActmemDropTool);

fn parameters() -> Value {
    json!({"type":"object","properties":{
        "section":{"type":"string","enum":["Goal","Open","Next","Constraints","Pointers"]},
        "item_index":{"type":"integer","minimum":0},
        "base_revision":{"type":"integer","minimum":0}
    },"required":["section","item_index","base_revision"]})
}

#[async_trait]
impl Tool for ActmemCompleteTool {
    fn name(&self) -> &str {
        "actmem_complete"
    }

    fn description(&self) -> &str {
        "Complete and remove one Open item by base_revision, section, and zero-based item_index."
    }

    fn parameters(&self) -> Value {
        parameters()
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let request: ActmemItemRequest = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("memory provider unavailable".to_string()))?;
        let response = provider
            .actmem_complete(request)
            .await
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        serde_json::to_string(&response)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}

#[async_trait]
impl Tool for ActmemDropTool {
    fn name(&self) -> &str {
        "actmem_drop"
    }

    fn description(&self) -> &str {
        "Drop one ACTMEM Work item by base_revision, section, and zero-based item_index."
    }

    fn parameters(&self) -> Value {
        parameters()
    }

    async fn execute(&self, args: Value) -> Result<String, ToolError> {
        let request: ActmemItemRequest = serde_json::from_value(args)
            .map_err(|error| ToolError::InvalidArguments(error.to_string()))?;
        let provider = self
            .provider
            .as_ref()
            .ok_or_else(|| ToolError::ExecutionFailed("memory provider unavailable".to_string()))?;
        let response = provider
            .actmem_drop(request)
            .await
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))?;
        serde_json::to_string(&response)
            .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}
