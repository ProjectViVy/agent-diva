//! CORE tools for deterministic search and task-local activation of deferred
//! MCP/custom tools.

use agent_diva_tooling::{DeferredToolActivationHandle, Tool, ToolError};
use async_trait::async_trait;
use serde_json::Value;

/// Search the currently authorized deferred catalog.
pub struct ToolSearchTool {
    discovery: DeferredToolActivationHandle,
}

impl ToolSearchTool {
    pub fn new(discovery: DeferredToolActivationHandle) -> Self {
        Self { discovery }
    }
}

#[async_trait]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str {
        "tool_search"
    }

    fn description(&self) -> &str {
        "Search authorized deferred MCP and custom tools by name, description, and schema keywords."
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Keywords to search for; empty returns the first catalog entries." },
                "limit": { "type": "integer", "minimum": 1, "maximum": 20, "default": 8 }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, args: Value) -> agent_diva_tooling::Result<String> {
        let query = args
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| ToolError::InvalidParams("Missing 'query' parameter".to_string()))?;
        let limit = args
            .get("limit")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .unwrap_or(8)
            .clamp(1, 20);
        let results = self.discovery.search(query, limit);
        serde_json::to_string(&serde_json::json!({
            "query": query,
            "results": results,
            "activated": self.discovery.active_names(),
        }))
        .map_err(|error| ToolError::ExecutionFailed(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_tooling::{ToolRegistry, ToolSchemaPartition};
    use std::sync::Arc;

    struct DeferredFixture;

    #[async_trait]
    impl Tool for DeferredFixture {
        fn name(&self) -> &str {
            "calendar_lookup"
        }

        fn description(&self) -> &str {
            "Look up calendar events by date"
        }

        fn parameters(&self) -> Value {
            serde_json::json!({
                "type": "object",
                "properties": { "date": { "type": "string" } }
            })
        }

        async fn execute(&self, _args: Value) -> agent_diva_tooling::Result<String> {
            Ok("calendar result".to_string())
        }
    }

    #[tokio::test]
    async fn search_auto_activates_and_exposes_tool() {
        let mut registry = ToolRegistry::new();
        let discovery = registry.deferred_tool_activation_handle();
        registry.register_in_partition(Arc::new(DeferredFixture), ToolSchemaPartition::Deferred);
        let search = ToolSearchTool::new(discovery.clone());
        let result = search
            .execute(serde_json::json!({ "query": "calendar" }))
            .await
            .unwrap();
        assert!(result.contains("calendar_lookup"));
        assert!(result.contains("activated"));
        assert_eq!(discovery.active_names(), vec!["calendar_lookup"]);
        assert!(registry
            .get_definition_set()
            .definitions
            .iter()
            .any(|definition| definition["function"]["name"] == "calendar_lookup"));
        assert_eq!(
            registry
                .execute("calendar_lookup", serde_json::json!({ "date": "today" }))
                .await
                .unwrap(),
            "calendar result"
        );
    }
}
