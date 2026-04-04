//! Agent loop: the core processing engine

use agent_diva_core::bus::{AgentEvent, InboundMessage, MessageBus, OutboundMessage};
use agent_diva_core::config::MCPServerConfig;
use agent_diva_core::cron::CronService;
use agent_diva_core::security::{SecurityConfig, SecurityLevel, SecurityPolicy};
use agent_diva_core::error_context::ErrorContext;
use agent_diva_core::session::SessionManager;
use agent_diva_files::{FileConfig, FileManager};
use agent_diva_providers::LLMProvider;
use agent_diva_tools::{
    load_mcp_tools_sync, CronTool, EditFileTool, ExecTool, ListDirTool, ReadFileTool, SpawnTool,
    ToolError, ToolRegistry, WriteFileTool,
};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::consolidation;
use crate::context::{ContextBuilder, SoulContextSettings};
use crate::runtime_control::RuntimeControlCommand;
use crate::subagent::SubagentManager;
use crate::tool_config::network::NetworkToolConfig;

mod loop_runtime_control;
mod loop_tools;
mod loop_turn;

/// Configuration for tool setup
#[derive(Clone)]
pub struct ToolConfig {
    /// Network tool runtime config
    pub network: NetworkToolConfig,
    /// Shell execution timeout in seconds
    pub exec_timeout: u64,
    /// Whether to restrict file access to workspace
    pub restrict_to_workspace: bool,
    /// Configured MCP servers
    pub mcp_servers: HashMap<String, MCPServerConfig>,
    /// Optional cron service for scheduling tools
    pub cron_service: Option<Arc<CronService>>,
    /// Soul context settings
    pub soul_context: SoulContextSettings,
    /// Whether to append transparent notifications on soul updates
    pub notify_on_soul_change: bool,
    /// Governance behavior for soul evolution transparency
    pub soul_governance: SoulGovernanceSettings,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            network: NetworkToolConfig::default(),
            exec_timeout: 60,
            restrict_to_workspace: false,
            mcp_servers: HashMap::new(),
            cron_service: None,
            soul_context: SoulContextSettings::default(),
            notify_on_soul_change: true,
            soul_governance: SoulGovernanceSettings::default(),
        }
    }
}

/// Runtime soft-governance settings for soul evolution.
#[derive(Clone, Debug)]
pub struct SoulGovernanceSettings {
    /// Rolling window in seconds for "frequent changes" hints.
    pub frequent_change_window_secs: u64,
    /// Minimum number of soul-changing turns in window to trigger hints.
    pub frequent_change_threshold: usize,
    /// Add a confirmation hint when SOUL.md changes.
    pub boundary_confirmation_hint: bool,
}

impl Default for SoulGovernanceSettings {
    fn default() -> Self {
        Self {
            frequent_change_window_secs: 600,
            frequent_change_threshold: 3,
            boundary_confirmation_hint: true,
        }
    }
}

/// The agent loop is the core processing engine
pub struct AgentLoop {
    bus: MessageBus,
    provider: Arc<dyn LLMProvider>,
    #[allow(dead_code)]
    workspace: PathBuf,
    #[allow(dead_code)]
    model: String,
    max_iterations: usize,
    memory_window: usize,
    context: ContextBuilder,
    sessions: SessionManager,
    tools: ToolRegistry,
    subagent_manager: Arc<SubagentManager>,
    runtime_control_rx: Option<mpsc::UnboundedReceiver<RuntimeControlCommand>>,
    cancelled_sessions: HashSet<String>,
    notify_on_soul_change: bool,
    soul_governance: SoulGovernanceSettings,
    soul_change_turns: VecDeque<Instant>,
    file_manager: FileManager,
}

impl AgentLoop {
    /// Create a new agent loop
    pub async fn new(
        bus: MessageBus,
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        model: Option<String>,
        max_iterations: Option<usize>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let model = model.unwrap_or_else(|| provider.get_default_model());
        let mut context = ContextBuilder::with_skills(workspace.clone(), None);
        context.set_soul_settings(SoulContextSettings::default());
        let sessions = SessionManager::new(workspace.clone());
        let tools = ToolRegistry::new();

        let subagent_manager = Arc::new(SubagentManager::new(
            provider.clone(),
            workspace.clone(),
            bus.clone(),
            Some(model.clone()),
            NetworkToolConfig::default(),
            None,
            false,
        ));

        // Initialize file manager for attachment handling
        let storage_path = dirs::data_local_dir()
            .map(|p| p.join("agent-diva").join("files"))
            .unwrap_or_else(|| PathBuf::from(".agent-diva/files"));
        let file_config = FileConfig::with_path(&storage_path);
        let file_manager = FileManager::new(file_config).await?;

        Ok(Self {
            bus,
            provider,
            workspace,
            model,
            max_iterations: max_iterations.unwrap_or(20),
            memory_window: consolidation::DEFAULT_MEMORY_WINDOW,
            context,
            sessions,
            tools,
            subagent_manager,
            runtime_control_rx: None,
            cancelled_sessions: HashSet::new(),
            notify_on_soul_change: true,
            soul_governance: SoulGovernanceSettings::default(),
            soul_change_turns: VecDeque::new(),
            file_manager,
        })
    }

    /// Create a new agent loop with tool configuration
    pub async fn with_tools(
        bus: MessageBus,
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        model: Option<String>,
        max_iterations: Option<usize>,
        tool_config: ToolConfig,
        runtime_control_rx: Option<mpsc::UnboundedReceiver<RuntimeControlCommand>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let model = model.unwrap_or_else(|| provider.get_default_model());
        let mut context = ContextBuilder::with_skills(workspace.clone(), None);
        context.set_soul_settings(tool_config.soul_context.clone());
        let sessions = SessionManager::new(workspace.clone());
        let mut tools = ToolRegistry::new();

        let subagent_manager = Arc::new(SubagentManager::new(
            provider.clone(),
            workspace.clone(),
            bus.clone(),
            Some(model.clone()),
            tool_config.network.clone(),
            Some(tool_config.exec_timeout),
            tool_config.restrict_to_workspace,
        ));

        // Register spawn tool
        let sm = subagent_manager.clone();
        tools.register(Arc::new(SpawnTool::new(
            move |task, label, channel, chat_id| {
                let sm = sm.clone();
                async move {
                    sm.spawn(task, label, channel, chat_id)
                        .await
                        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
                }
            },
        )));

        // Register file system tools with SecurityPolicy
        let security_config = if tool_config.restrict_to_workspace {
            SecurityConfig {
                level: SecurityLevel::Standard,
                workspace_only: true,
                ..SecurityConfig::default()
            }
        } else {
            SecurityConfig::default()
        };
        let security = Arc::new(SecurityPolicy::with_config(workspace.clone(), security_config));
        tools.register(Arc::new(ReadFileTool::new(security.clone())));
        tools.register(Arc::new(WriteFileTool::new(security.clone())));
        tools.register(Arc::new(EditFileTool::new(security.clone())));
        tools.register(Arc::new(ListDirTool::new(security)));

        // Register shell tool
        tools.register(Arc::new(ExecTool::with_config(
            tool_config.exec_timeout,
            Some(workspace.clone()),
            tool_config.restrict_to_workspace,
        )));

        // Register web tools
        Self::register_web_tools(&mut tools, &tool_config.network);

        // Register MCP tools discovered from configured servers
        for mcp_tool in load_mcp_tools_sync(&tool_config.mcp_servers) {
            tools.register(mcp_tool);
        }

        // Register cron tool when scheduling is configured
        if let Some(cron_service) = tool_config.cron_service.clone() {
            tools.register(Arc::new(CronTool::new(cron_service)));
        }

        // Initialize file manager for attachment handling
        let storage_path = dirs::data_local_dir()
            .map(|p| p.join("agent-diva").join("files"))
            .unwrap_or_else(|| PathBuf::from(".agent-diva/files"));
        let file_config = FileConfig::with_path(&storage_path);
        let file_manager = FileManager::new(file_config).await?;

        Ok(Self {
            bus,
            provider,
            workspace,
            model,
            max_iterations: max_iterations.unwrap_or(20),
            memory_window: consolidation::DEFAULT_MEMORY_WINDOW,
            context,
            sessions,
            tools,
            subagent_manager,
            runtime_control_rx,
            cancelled_sessions: HashSet::new(),
            notify_on_soul_change: tool_config.notify_on_soul_change,
            soul_governance: tool_config.soul_governance,
            soul_change_turns: VecDeque::new(),
            file_manager,
        })
    }

    /// Run the agent loop, processing messages from the bus
    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Agent loop started");

        // Take the inbound receiver
        let Some(mut inbound_rx) = self.bus.take_inbound_receiver().await else {
            error!("Failed to take inbound receiver");
            return Err("Inbound receiver already taken".into());
        };

        loop {
            if let Some(control_rx) = self.runtime_control_rx.as_mut() {
                tokio::select! {
                    control = control_rx.recv() => {
                        match control {
                            Some(cmd) => self.handle_runtime_control_command(cmd).await,
                            None => {
                                info!("Runtime control channel closed");
                                self.runtime_control_rx = None;
                            }
                        }
                    }
                    maybe_msg = inbound_rx.recv() => {
                        match maybe_msg {
                            Some(msg) => self.handle_inbound(msg).await,
                            None => {
                                info!("Message bus closed, stopping agent loop");
                                break;
                            }
                        }
                    }
                }
            } else {
                match tokio::time::timeout(std::time::Duration::from_secs(1), inbound_rx.recv())
                    .await
                {
                    Ok(Some(msg)) => self.handle_inbound(msg).await,
                    Ok(None) => {
                        info!("Message bus closed, stopping agent loop");
                        break;
                    }
                    Err(_) => continue,
                }
            }
        }

        info!("Agent loop stopped");
        Ok(())
    }

    async fn handle_inbound(&mut self, msg: InboundMessage) {
        debug!("Received message from {}:{}", msg.channel, msg.chat_id);
        let event_msg = msg.clone();
        match self.process_inbound_message(msg, None).await {
            Ok(Some(response)) => {
                if let Err(e) = self.bus.publish_outbound(response) {
                    error!("Failed to publish response: {}", e);
                }
            }
            Ok(None) => debug!("No response needed"),
            Err(e) => {
                let error_message = format!("Failed to process message: {}", e);
                let ctx = ErrorContext::new("handle_inbound", &error_message)
                    .with_metadata("channel", event_msg.channel.clone())
                    .with_metadata("chat_id", event_msg.chat_id.clone())
                    .with_metadata("sender_id", event_msg.sender_id.clone());
                error!("{}", ctx.to_detailed_string());
                self.emit_error_event(&event_msg, None, error_message);
            }
        }
    }

    /// Process a single inbound message
    pub async fn process_inbound_message(
        &mut self,
        msg: InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
    ) -> Result<Option<OutboundMessage>, Box<dyn std::error::Error>> {
        let trace_id = Uuid::new_v4().to_string();
        use tracing::Instrument;
        let span = tracing::info_span!("AgentSpan", trace_id = %trace_id);

        self.process_inbound_message_inner(msg, event_tx, trace_id)
            .instrument(span)
            .await
    }

    /// Process a message directly (for CLI or testing)
    pub async fn process_direct(
        &mut self,
        content: impl Into<String>,
        _session_key: impl Into<String>,
        channel: impl Into<String>,
        chat_id: impl Into<String>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let content = content.into();
        let channel = channel.into();
        let chat_id = chat_id.into();

        let msg = InboundMessage::new(channel, "user", chat_id, content);

        let response = self.process_inbound_message(msg, None).await?;
        Ok(response
            .map(|r| {
                let content = r.content;
                if let Some(reasoning) = r.reasoning_content {
                    if !reasoning.is_empty() {
                        return format!("<think>\n{}\n</think>\n\n{}", reasoning, content);
                    }
                }
                content
            })
            .unwrap_or_default())
    }

    /// Process a message directly and emit streaming events for UI consumers.
    pub async fn process_direct_stream(
        &mut self,
        content: impl Into<String>,
        _session_key: impl Into<String>,
        channel: impl Into<String>,
        chat_id: impl Into<String>,
        event_tx: mpsc::UnboundedSender<AgentEvent>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let content = content.into();
        let channel = channel.into();
        let chat_id = chat_id.into();

        let msg = InboundMessage::new(channel, "user", chat_id, content);

        match self.process_inbound_message(msg, Some(&event_tx)).await {
            Ok(response) => Ok(response.map(|r| r.content).unwrap_or_default()),
            Err(err) => {
                let _ = event_tx.send(AgentEvent::Error {
                    message: err.to_string(),
                });
                Err(err)
            }
        }
    }

    fn is_frequent_soul_change_turn(&mut self) -> bool {
        let window = Duration::from_secs(self.soul_governance.frequent_change_window_secs.max(1));
        let now = Instant::now();
        self.soul_change_turns.push_back(now);
        while let Some(front) = self.soul_change_turns.front().copied() {
            if now.duration_since(front) > window {
                self.soul_change_turns.pop_front();
            } else {
                break;
            }
        }
        self.soul_change_turns.len() >= self.soul_governance.frequent_change_threshold.max(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_providers::{
        LLMResponse, LiteLLMClient, Message, ProviderError, ProviderEventStream, ProviderResult,
    };
    use async_trait::async_trait;
    use futures::stream;
    use tokio::time::{timeout, Duration};

    struct FailingStreamProvider;

    #[async_trait]
    impl LLMProvider for FailingStreamProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            Err(ProviderError::ApiError(
                "chat should not be used".to_string(),
            ))
        }

        async fn chat_stream(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<ProviderEventStream> {
            Ok(Box::pin(stream::iter(vec![Err(ProviderError::ApiError(
                "simulated stream failure".to_string(),
            ))])))
        }

        fn get_default_model(&self) -> String {
            "test-model".to_string()
        }
    }

    #[tokio::test]
    async fn test_agent_loop_creation() {
        let bus = MessageBus::new();
        let provider = Arc::new(LiteLLMClient::default());
        let workspace = PathBuf::from("/tmp/test");
        let agent = AgentLoop::new(bus, provider, workspace, None, None);
        assert_eq!(agent.max_iterations, 20);
    }

    #[tokio::test]
    async fn test_process_direct() {
        let bus = MessageBus::new();
        let provider = Arc::new(LiteLLMClient::default());
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut agent = AgentLoop::new(bus, provider, workspace, None, Some(1));

        // This will fail to connect to LLM, but tests the structure
        let result = agent
            .process_direct("Hello", "cli:test", "cli", "test")
            .await;

        // We expect an error since we don't have a real LLM connection
        assert!(result.is_err());
    }

    #[test]
    fn test_soul_governance_defaults_are_non_zero() {
        let cfg = SoulGovernanceSettings::default();
        assert!(cfg.frequent_change_window_secs > 0);
        assert!(cfg.frequent_change_threshold > 0);
    }

    #[tokio::test]
    async fn test_handle_inbound_emits_error_event_on_provider_failure() {
        let bus = MessageBus::new();
        let mut event_rx = bus.subscribe_events();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut agent = AgentLoop::new(bus.clone(), provider, workspace, None, Some(1));
        let msg = InboundMessage::new("gui", "user", "chat-1", "Hello");

        agent.handle_inbound(msg).await;

        let error_event = timeout(Duration::from_secs(1), async {
            loop {
                let bus_event = event_rx.recv().await.unwrap();
                if let AgentEvent::Error { message } = bus_event.event {
                    break (bus_event.channel, bus_event.chat_id, message);
                }
            }
        })
        .await
        .expect("timed out waiting for error event");

        assert_eq!(error_event.0, "gui");
        assert_eq!(error_event.1, "chat-1");
        assert!(error_event.2.contains("simulated stream failure"));
    }
}
