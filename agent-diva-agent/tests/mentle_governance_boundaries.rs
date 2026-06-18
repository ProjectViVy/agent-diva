use agent_diva_agent::tool_config::mentle::{MentleToolMode, MentleToolRuntimeConfig};
use agent_diva_agent::{AgentLoop, AgentLoopToolSet, ToolConfig};
use agent_diva_core::bus::MessageBus;
use agent_diva_files::{FileConfig, FileManager};
use agent_diva_providers::{LLMProvider, LLMResponse, Message, ProviderResult};
use agent_diva_tooling::{Tool, ToolError, ToolRegistry};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

struct NoopProvider;

#[async_trait]
impl LLMProvider for NoopProvider {
    async fn chat(
        &self,
        _messages: Vec<Message>,
        _tools: Option<Vec<Value>>,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        Ok(LLMResponse {
            content: Some(String::new()),
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: HashMap::new(),
            reasoning_content: None,
        })
    }

    fn get_default_model(&self) -> String {
        "test-model".to_string()
    }
}

struct NamedTool {
    name: &'static str,
}

#[async_trait]
impl Tool for NamedTool {
    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        "test tool"
    }

    fn parameters(&self) -> Value {
        serde_json::json!({
            "type": "object",
            "properties": {},
            "required": []
        })
    }

    async fn execute(&self, _args: Value) -> Result<String, ToolError> {
        Ok("ok".to_string())
    }
}

async fn build_agent_with_enabled_mentle_tools(
    tools: impl IntoIterator<Item = Arc<dyn Tool>>,
) -> (AgentLoop, tempfile::TempDir) {
    let temp = tempfile::tempdir().expect("temp workspace");
    let workspace = temp.path().to_path_buf();
    let file_manager = Arc::new(
        FileManager::new(FileConfig::with_path(temp.path().join("files")))
            .await
            .expect("file manager"),
    );
    let mut registry = ToolRegistry::new();
    for tool in tools {
        registry.register(tool);
    }
    let mut config = ToolConfig::default();
    config.builtin.mentle = true;
    config.mentle = MentleToolRuntimeConfig {
        enabled: true,
        mode: MentleToolMode::ReadOnly,
        allowed_tools: Vec::new(),
    };

    AgentLoop::with_toolset(
        MessageBus::new(),
        Arc::new(NoopProvider),
        workspace,
        None,
        Some(1),
        AgentLoopToolSet { registry, config },
        None,
        file_manager,
    )
    .await
    .map(|agent| (agent, temp))
    .expect("agent with toolset")
}

#[tokio::test]
async fn enabled_mentle_runtime_does_not_expose_default_recall_routing_in_governance_prompt() {
    let (agent, _temp) = build_agent_with_enabled_mentle_tools([
        Arc::new(NamedTool {
            name: "memtle_status",
        }) as Arc<dyn Tool>,
        Arc::new(NamedTool {
            name: "memtle_search",
        }) as Arc<dyn Tool>,
    ])
    .await;

    let prompt = agent.build_system_prompt();
    let lower_prompt = prompt.to_lowercase();

    assert!(agent.mentle_active());
    assert!(!prompt.contains("memtle_search"));
    assert!(!prompt.contains("memtle_kg_query"));
    assert!(!prompt.contains("memtle_*"));
    assert!(!lower_prompt.contains("mentle recall"));
    assert!(!lower_prompt.contains("palace memory"));
}

#[tokio::test]
async fn enabled_mentle_search_without_status_is_not_a_governance_runtime_surface() {
    let (agent, _temp) = build_agent_with_enabled_mentle_tools([Arc::new(NamedTool {
        name: "memtle_search",
    }) as Arc<dyn Tool>])
    .await;

    let prompt = agent.build_system_prompt();

    assert!(!agent.mentle_active());
    assert!(!prompt.contains("memtle_search"));
    assert!(!prompt.contains("memtle_kg_query"));
}
