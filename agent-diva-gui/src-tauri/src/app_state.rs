use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use agent_diva_agent::{AgentLoop, ToolConfig};
use agent_diva_core::bus::MessageBus;
use std::collections::HashMap;
use agent_diva_providers::LiteLLMClient;
use agent_diva_providers::base::LLMProvider;

pub struct AgentState {
    pub agent: Arc<Mutex<Option<AgentLoop>>>,
}

impl AgentState {
    pub fn new() -> Self {
        // Initially, we might not have configuration, so we start with None
        // Or we can try to load from environment/default
        
        let agent = Self::create_agent(None, None, None).ok();
        
        Self {
            agent: Arc::new(Mutex::new(agent)),
        }
    }
    
    pub fn create_agent(
        api_base: Option<String>,
        api_key: Option<String>,
        model: Option<String>,
    ) -> Result<AgentLoop, String> {
        let bus = MessageBus::new();
        
        // Default to DeepSeek
        let base = api_base.or_else(|| std::env::var("LITELLM_API_BASE").ok())
            .or_else(|| Some("https://api.deepseek.com/v1".to_string()));
            
        let key = api_key.or_else(|| std::env::var("LITELLM_API_KEY").ok());
        
        let model_name = model.or_else(|| std::env::var("LITELLM_MODEL").ok())
            .unwrap_or_else(|| "deepseek-chat".to_string());

        let provider = Arc::new(LiteLLMClient::new(
            key,
            base,
            model_name.clone(),
            None,
            None
        ));
        
        // Use a dedicated workspace directory for the GUI agent
        let mut workspace = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        workspace.push(".agent-diva-gui");
        std::fs::create_dir_all(&workspace).map_err(|e| e.to_string())?;
        
        let tool_config = ToolConfig {
            brave_api_key: std::env::var("BRAVE_API_KEY").ok(),
            exec_timeout: 120,
            restrict_to_workspace: false,
            mcp_servers: HashMap::new(),
        };

        let agent = AgentLoop::with_tools(
            bus,
            provider,
            workspace,
            Some(model_name),
            Some(20), // Max iterations
            tool_config,
        );
        
        Ok(agent)
    }
    
    pub async fn reconfigure(&self, api_base: Option<String>, api_key: Option<String>, model: Option<String>) -> Result<(), String> {
        let new_agent = Self::create_agent(api_base, api_key, model)?;
        let mut agent_lock = self.agent.lock().await;
        *agent_lock = Some(new_agent);
        Ok(())
    }
}
