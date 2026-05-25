//! Agent loop: the core processing engine

use agent_diva_core::bus::{AgentEvent, InboundMessage, MessageBus, OutboundMessage};
use agent_diva_core::config::MCPServerConfig;
use agent_diva_core::cron::CronService;
use agent_diva_core::error_context::ErrorContext;
use agent_diva_core::memory::{MemoryProvider, SessionEndRequest};
use agent_diva_core::session::SessionManager;
use agent_diva_files::{FileConfig, FileManager};
use agent_diva_providers::LLMProvider;
use agent_diva_tooling::{Tool, ToolError, ToolRegistry};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use crate::consolidation;
use crate::context::{ContextBuilder, SoulContextSettings};
#[cfg(feature = "mentle")]
use crate::mentle_runtime::MentleRuntime;
use crate::runtime_control::RuntimeControlCommand;
use crate::subagent::SubagentManager;
use crate::tool_assembly::{SubagentSpawner, ToolAssembly};
use crate::tool_config::builtin::BuiltInToolsConfig;
use crate::tool_config::mentle::MentleToolRuntimeConfig;
use crate::tool_config::network::NetworkToolConfig;

mod loop_runtime_control;
mod loop_tools;
mod loop_turn;

/// Configuration for tool setup
#[derive(Clone)]
pub struct ToolConfig {
    /// Built-in tool toggles.
    pub builtin: BuiltInToolsConfig,
    /// Network tool runtime config
    pub network: NetworkToolConfig,
    /// Mentle tool selection policy.
    pub mentle: MentleToolRuntimeConfig,
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
            builtin: BuiltInToolsConfig::default(),
            network: NetworkToolConfig::default(),
            mentle: MentleToolRuntimeConfig::default(),
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
    tool_config: ToolConfig,
    tools: ToolRegistry,
    subagent_manager: Arc<SubagentManager>,
    runtime_control_rx: Option<mpsc::UnboundedReceiver<RuntimeControlCommand>>,
    cancelled_sessions: HashSet<String>,
    notify_on_soul_change: bool,
    soul_governance: SoulGovernanceSettings,
    soul_change_turns: VecDeque<Instant>,
    file_manager: Arc<FileManager>,
    /// Memory provider boundary for prefetch, sync_turn, and shutdown hooks.
    memory_provider: Arc<dyn MemoryProvider>,
    custom_tools: Vec<Arc<dyn Tool>>,
    mentle_active: bool,
    #[cfg(feature = "mentle")]
    #[allow(dead_code)]
    mentle_runtime: Option<MentleRuntime>,
}

pub struct AgentLoopToolSet {
    pub registry: ToolRegistry,
    pub config: ToolConfig,
}

pub struct AgentLoopToolSetBuilder {
    registry: ToolRegistry,
    config: ToolConfig,
}

impl AgentLoopToolSetBuilder {
    pub fn new(config: ToolConfig) -> Self {
        Self {
            registry: ToolRegistry::new(),
            config,
        }
    }

    pub fn with_tool(mut self, tool: Arc<dyn Tool>) -> Self {
        self.registry.register(tool);
        self
    }

    pub fn with_tools(mut self, tools: Vec<Arc<dyn Tool>>) -> Self {
        for tool in tools {
            self.registry.register(tool);
        }
        self
    }

    pub fn build(self) -> AgentLoopToolSet {
        AgentLoopToolSet {
            registry: self.registry,
            config: self.config,
        }
    }
}

impl AgentLoopToolSet {
    pub fn builder(config: ToolConfig) -> AgentLoopToolSetBuilder {
        AgentLoopToolSetBuilder::new(config)
    }
}

struct SubagentManagerSpawner {
    manager: Arc<SubagentManager>,
}

#[async_trait::async_trait]
impl SubagentSpawner for SubagentManagerSpawner {
    async fn spawn(
        &self,
        task: String,
        label: Option<String>,
        channel: String,
        chat_id: String,
    ) -> Result<String, ToolError> {
        self.manager
            .spawn(task, label, channel, chat_id)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
    }
}

fn build_agent_tools(
    workspace: PathBuf,
    tool_config: &ToolConfig,
    spawner: Arc<dyn SubagentSpawner>,
    file_manager: Arc<FileManager>,
    custom_tools: Vec<Arc<dyn Tool>>,
    cron_service: Option<Arc<CronService>>,
) -> ToolRegistry {
    let mut assembly = ToolAssembly::new(workspace)
        .builtin(tool_config.builtin.clone())
        .with_network_config(tool_config.network.clone())
        .with_exec_timeout(tool_config.exec_timeout)
        .restrict_to_workspace(tool_config.restrict_to_workspace)
        .mcp_servers(tool_config.mcp_servers.clone())
        .with_subagent_spawner(spawner)
        .with_file_manager(file_manager)
        .with_tools(custom_tools);

    if let Some(cron_service) = cron_service {
        assembly = assembly.with_cron_service(cron_service);
    }

    assembly.build()
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
            BuiltInToolsConfig::default().for_subagent(),
            NetworkToolConfig::default(),
            None,
            false,
            HashMap::new(),
        ));

        // Initialize file manager for attachment handling
        let storage_path = dirs::data_local_dir()
            .map(|p| p.join("agent-diva").join("files"))
            .unwrap_or_else(|| PathBuf::from(".agent-diva/files"));
        let file_config = FileConfig::with_path(&storage_path);
        let file_manager = Arc::new(FileManager::new(file_config).await?);

        let memory_provider: Arc<dyn MemoryProvider> =
            Arc::new(agent_diva_core::memory::MemoryManager::new(&workspace));

        Ok(Self {
            bus,
            provider,
            workspace,
            model,
            max_iterations: max_iterations.unwrap_or(20),
            memory_window: consolidation::DEFAULT_MEMORY_WINDOW,
            context,
            sessions,
            tool_config: ToolConfig::default(),
            tools,
            subagent_manager,
            runtime_control_rx: None,
            cancelled_sessions: HashSet::new(),
            notify_on_soul_change: true,
            soul_governance: SoulGovernanceSettings::default(),
            soul_change_turns: VecDeque::new(),
            file_manager,
            memory_provider,
            custom_tools: Vec::new(),
            mentle_active: false,
            #[cfg(feature = "mentle")]
            mentle_runtime: None,
        })
    }

    /// Get the file manager
    pub fn file_manager(&self) -> Arc<FileManager> {
        self.file_manager.clone()
    }

    /// Create a new agent loop with tool configuration
    #[allow(clippy::too_many_arguments)]
    pub async fn with_tools(
        bus: MessageBus,
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        model: Option<String>,
        max_iterations: Option<usize>,
        tool_config: ToolConfig,
        runtime_control_rx: Option<mpsc::UnboundedReceiver<RuntimeControlCommand>>,
        file_manager: Arc<FileManager>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_tools_and_memory_provider(
            bus,
            provider,
            workspace,
            model,
            max_iterations,
            tool_config,
            runtime_control_rx,
            file_manager,
            None,
        )
        .await
    }

    /// Create a new agent loop with tool configuration and a custom memory provider.
    ///
    /// When `memory_provider` is `None`, a default `MemoryManager` is used.
    #[allow(clippy::too_many_arguments)]
    pub async fn with_tools_and_memory_provider(
        bus: MessageBus,
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        model: Option<String>,
        max_iterations: Option<usize>,
        tool_config: ToolConfig,
        runtime_control_rx: Option<mpsc::UnboundedReceiver<RuntimeControlCommand>>,
        file_manager: Arc<FileManager>,
        memory_provider: Option<Arc<dyn MemoryProvider>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        Self::with_tools_and_memory_provider_inner(
            bus,
            provider,
            workspace,
            model,
            max_iterations,
            tool_config,
            runtime_control_rx,
            file_manager,
            memory_provider,
            #[cfg(feature = "mentle")]
            None,
        )
        .await
    }

    #[allow(clippy::too_many_arguments)]
    async fn with_tools_and_memory_provider_inner(
        bus: MessageBus,
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        model: Option<String>,
        max_iterations: Option<usize>,
        tool_config: ToolConfig,
        runtime_control_rx: Option<mpsc::UnboundedReceiver<RuntimeControlCommand>>,
        file_manager: Arc<FileManager>,
        memory_provider: Option<Arc<dyn MemoryProvider>>,
        #[cfg(feature = "mentle")] mentle_runtime_override: Option<Option<MentleRuntime>>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let model = model.unwrap_or_else(|| provider.get_default_model());
        let mut context = ContextBuilder::with_skills(workspace.clone(), None);
        context.set_soul_settings(tool_config.soul_context.clone());
        let sessions = SessionManager::new(workspace.clone());

        let subagent_manager = Arc::new(SubagentManager::new(
            provider.clone(),
            workspace.clone(),
            bus.clone(),
            Some(model.clone()),
            tool_config.builtin.for_subagent(),
            tool_config.network.clone(),
            Some(tool_config.exec_timeout),
            tool_config.restrict_to_workspace,
            tool_config.mcp_servers.clone(),
        ));

        let spawner: Arc<dyn SubagentSpawner> = Arc::new(SubagentManagerSpawner {
            manager: subagent_manager.clone(),
        });

        #[cfg(feature = "mentle")]
        let (mentle_active, custom_tools, active_memory_provider, mentle_runtime) = {
            let mut active_memory_provider = memory_provider;
            let mut custom_tools: Vec<Arc<dyn Tool>> = Vec::new();
            let mut mentle_active = false;
            let mentle_requested =
                tool_config.builtin.mentle && tool_config.mentle.is_active_request();
            let mentle_runtime = if mentle_requested {
                if let Some(override_runtime) = mentle_runtime_override {
                    override_runtime
                } else {
                    MentleRuntime::try_build(&workspace, &tool_config.mentle).await
                }
            } else {
                None
            };

            if let Some(runtime) = &mentle_runtime {
                custom_tools = runtime.custom_tools();
                mentle_active = runtime.active();
                if mentle_requested && !mentle_active {
                    warn!("Mentle prompt disabled: runtime tools do not contain memtle_status");
                }
                if active_memory_provider.is_none() {
                    active_memory_provider = Some(runtime.memory_provider());
                }
            } else if mentle_requested {
                warn!(
                    "Mentle requested but runtime is unavailable; falling back to Markdown memory"
                );
            }

            (
                mentle_active,
                custom_tools,
                active_memory_provider,
                mentle_runtime,
            )
        };

        #[cfg(not(feature = "mentle"))]
        let (mentle_active, custom_tools, active_memory_provider) = {
            if tool_config.builtin.mentle && tool_config.mentle.is_active_request() {
                warn!("Mentle requested but the agent-diva-agent `mentle` feature is disabled");
            }
            (false, Vec::<Arc<dyn Tool>>::new(), memory_provider)
        };

        let memory_provider = active_memory_provider
            .unwrap_or_else(|| Arc::new(agent_diva_core::memory::MemoryManager::new(&workspace)));
        let mentle_tool_names = custom_tools
            .iter()
            .map(|tool| tool.name().to_string())
            .collect();
        context = context
            .with_memory_provider(memory_provider.clone())
            .with_mentle(mentle_active)
            .with_mentle_tools(mentle_tool_names);

        let tools = build_agent_tools(
            workspace.clone(),
            &tool_config,
            spawner.clone(),
            file_manager.clone(),
            custom_tools.clone(),
            tool_config.cron_service.clone(),
        );

        let mut agent = Self {
            bus,
            provider,
            workspace,
            model,
            max_iterations: max_iterations.unwrap_or(20),
            memory_window: consolidation::DEFAULT_MEMORY_WINDOW,
            context,
            sessions,
            tool_config: tool_config.clone(),
            tools,
            subagent_manager,
            runtime_control_rx,
            cancelled_sessions: HashSet::new(),
            notify_on_soul_change: tool_config.notify_on_soul_change,
            soul_governance: tool_config.soul_governance,
            soul_change_turns: VecDeque::new(),
            file_manager,
            memory_provider,
            custom_tools,
            mentle_active,
            #[cfg(feature = "mentle")]
            mentle_runtime,
        };

        if let Some(cron_service) = agent.tool_config.cron_service.clone() {
            agent.tools = build_agent_tools(
                agent.workspace.clone(),
                &agent.tool_config,
                Arc::new(SubagentManagerSpawner {
                    manager: agent.subagent_manager.clone(),
                }),
                agent.file_manager.clone(),
                agent.custom_tools.clone(),
                Some(cron_service),
            );
        }

        Ok(agent)
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn with_toolset(
        bus: MessageBus,
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        model: Option<String>,
        max_iterations: Option<usize>,
        toolset: AgentLoopToolSet,
        runtime_control_rx: Option<mpsc::UnboundedReceiver<RuntimeControlCommand>>,
        file_manager: Arc<FileManager>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let model = model.unwrap_or_else(|| provider.get_default_model());
        let mut context = ContextBuilder::with_skills(workspace.clone(), None);
        context.set_soul_settings(toolset.config.soul_context.clone());
        let mentle_active = toolset.registry.has("memtle_status");
        if toolset.config.builtin.mentle && !mentle_active {
            warn!("Mentle prompt disabled: supplied toolset does not contain memtle_status");
        }
        let sessions = SessionManager::new(workspace.clone());
        let subagent_manager = Arc::new(SubagentManager::new(
            provider.clone(),
            workspace.clone(),
            bus.clone(),
            Some(model.clone()),
            toolset.config.builtin.for_subagent(),
            toolset.config.network.clone(),
            Some(toolset.config.exec_timeout),
            toolset.config.restrict_to_workspace,
            toolset.config.mcp_servers.clone(),
        ));

        let memory_provider: Arc<dyn MemoryProvider> =
            Arc::new(agent_diva_core::memory::MemoryManager::new(&workspace));
        let mentle_tool_names = toolset
            .registry
            .tool_names()
            .into_iter()
            .filter(|name| name.starts_with("memtle_"))
            .collect();
        context = context
            .with_memory_provider(memory_provider.clone())
            .with_mentle(mentle_active)
            .with_mentle_tools(mentle_tool_names);

        Ok(Self {
            bus,
            provider,
            workspace,
            model,
            max_iterations: max_iterations.unwrap_or(20),
            memory_window: consolidation::DEFAULT_MEMORY_WINDOW,
            context,
            sessions,
            tool_config: toolset.config.clone(),
            tools: toolset.registry,
            subagent_manager,
            runtime_control_rx,
            cancelled_sessions: HashSet::new(),
            notify_on_soul_change: toolset.config.notify_on_soul_change,
            soul_governance: toolset.config.soul_governance,
            soul_change_turns: VecDeque::new(),
            file_manager,
            memory_provider,
            custom_tools: Vec::new(),
            mentle_active,
            #[cfg(feature = "mentle")]
            mentle_runtime: None,
        })
    }

    /// Whether Mentle prompt routing is active for this loop.
    pub fn mentle_active(&self) -> bool {
        self.mentle_active
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

        // Trigger session-end rhythm work with idempotency.
        match self
            .memory_provider
            .on_session_end(SessionEndRequest {
                workspace_root: self.workspace.clone(),
                session_id: Some("agent-loop-shutdown".to_string()),
            })
            .await
        {
            Ok(response) => {
                debug!("Session-end hook completed: {:?}", response.status);
            }
            Err(e) => {
                warn!("Session-end hook failed: {}", e);
            }
        }

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
    #[cfg(feature = "mentle")]
    use crate::mentle_runtime::{
        mentle_tool_from_definition, mentle_tool_metadata_from_definition,
        mentle_tools_from_definitions, MentleRuntime, MentleToolkitTool,
    };
    #[cfg(feature = "mentle")]
    use crate::tool_config::mentle::{MentleToolMode, MentleToolRuntimeConfig};
    use agent_diva_providers::{
        LLMResponse, LLMStreamEvent, LiteLLMClient, Message, ProviderError, ProviderEventStream,
        ProviderResult, ToolCallRequest,
    };
    use async_trait::async_trait;
    use futures::stream;
    use serde_json::Value;
    use std::sync::Mutex;
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

    #[derive(Default)]
    struct CapturingStreamProvider {
        captured_messages: Mutex<Vec<Vec<Message>>>,
    }

    #[async_trait]
    impl LLMProvider for CapturingStreamProvider {
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
            messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<ProviderEventStream> {
            self.captured_messages.lock().unwrap().push(messages);
            Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
                LLMResponse {
                    content: Some("done".to_string()),
                    tool_calls: Vec::new(),
                    finish_reason: "stop".to_string(),
                    usage: HashMap::new(),
                    reasoning_content: None,
                },
            ))])))
        }

        fn get_default_model(&self) -> String {
            "test-model".to_string()
        }
    }

    #[derive(Default)]
    struct ConsolidatingStreamProvider {
        captured_messages: Mutex<Vec<Vec<Message>>>,
    }

    #[async_trait]
    impl LLMProvider for ConsolidatingStreamProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            Ok(LLMResponse {
                content: None,
                tool_calls: vec![ToolCallRequest {
                    id: "save-memory-call".to_string(),
                    call_type: "function".to_string(),
                    name: "save_memory".to_string(),
                    arguments: HashMap::from([
                        (
                            "memory_update".to_string(),
                            serde_json::Value::String("Updated continuity.".to_string()),
                        ),
                        (
                            "history_entry".to_string(),
                            serde_json::Value::String("Recorded turn.".to_string()),
                        ),
                    ]),
                }],
                finish_reason: "tool_calls".to_string(),
                usage: HashMap::new(),
                reasoning_content: None,
            })
        }

        async fn chat_stream(
            &self,
            messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<ProviderEventStream> {
            self.captured_messages.lock().unwrap().push(messages);
            Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
                LLMResponse {
                    content: Some("done".to_string()),
                    tool_calls: Vec::new(),
                    finish_reason: "stop".to_string(),
                    usage: HashMap::new(),
                    reasoning_content: None,
                },
            ))])))
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

        async fn execute(&self, _args: Value) -> agent_diva_tooling::Result<String> {
            Ok("ok".to_string())
        }
    }

    struct NoopSpawner;

    #[async_trait]
    impl SubagentSpawner for NoopSpawner {
        async fn spawn(
            &self,
            _task: String,
            _label: Option<String>,
            _channel: String,
            _chat_id: String,
        ) -> Result<String, ToolError> {
            Ok("spawned".to_string())
        }
    }

    #[tokio::test]
    async fn test_agent_loop_creation() {
        let bus = MessageBus::new();
        let provider = Arc::new(LiteLLMClient::default());
        let workspace = PathBuf::from("/tmp/test");
        let agent = AgentLoop::new(bus, provider, workspace, None, None)
            .await
            .unwrap();
        assert_eq!(agent.max_iterations, 20);
    }

    #[tokio::test]
    async fn test_process_direct() {
        let bus = MessageBus::new();
        let provider = Arc::new(LiteLLMClient::default());
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut agent = AgentLoop::new(bus, provider, workspace, None, Some(1))
            .await
            .unwrap();

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

        let mut agent = AgentLoop::new(bus.clone(), provider, workspace, None, Some(1))
            .await
            .unwrap();
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

    // ── memory provider lifecycle wiring tests (Task 6) ──────────────

    #[tokio::test]
    async fn test_with_toolset_missing_memtle_tool_disables_prompt() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut config = ToolConfig::default();
        config.builtin.mentle = true;
        let toolset = AgentLoopToolSet {
            registry: ToolRegistry::new(),
            config,
        };

        let agent = AgentLoop::with_toolset(
            bus,
            provider,
            workspace,
            None,
            Some(1),
            toolset,
            None,
            file_manager,
        )
        .await
        .unwrap();

        assert!(!agent.mentle_active());
        assert!(!agent
            .context
            .build_system_prompt()
            .contains("L2 Palace Memory"));
    }

    #[tokio::test]
    async fn test_with_toolset_memtle_status_enables_prompt() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(NamedTool {
            name: "memtle_status",
        }));
        let toolset = AgentLoopToolSet {
            registry,
            config: ToolConfig::default(),
        };

        let agent = AgentLoop::with_toolset(
            bus,
            provider,
            workspace,
            None,
            Some(1),
            toolset,
            None,
            file_manager,
        )
        .await
        .unwrap();

        assert!(agent.mentle_active());
        assert!(agent
            .context
            .build_system_prompt()
            .contains("L2 Palace Memory"));
    }

    #[tokio::test]
    async fn test_with_toolset_status_only_prompt_omits_search_tool_name() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(NamedTool {
            name: "memtle_status",
        }));
        let toolset = AgentLoopToolSet {
            registry,
            config: ToolConfig::default(),
        };

        let agent = AgentLoop::with_toolset(
            bus,
            provider,
            workspace,
            None,
            Some(1),
            toolset,
            None,
            file_manager,
        )
        .await
        .unwrap();

        let prompt = agent.context.build_system_prompt();

        assert!(agent.mentle_active());
        assert!(prompt.contains("L2 Palace Memory"));
        assert!(!prompt.contains("memtle_search"));
        assert!(!prompt.contains("memtle_kg_query"));
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_runtime_active_matches_registered_status_tool() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut config = ToolConfig::default();
        config.builtin.mentle = true;
        config.mentle = MentleToolRuntimeConfig {
            enabled: true,
            mode: MentleToolMode::Full,
            allowed_tools: Vec::new(),
        };
        let toolkit = memtle::toolkit::MemtleToolkit::open(workspace.join("palace.db"))
            .await
            .unwrap();
        let runtime = MentleRuntime::from_parts_for_test(
            Arc::new(tokio::sync::Mutex::new(toolkit)),
            Arc::new(agent_diva_core::memory::MemoryManager::new(&workspace)),
            vec![
                Arc::new(NamedTool {
                    name: "memtle_status",
                }),
                Arc::new(NamedTool {
                    name: "memtle_search",
                }),
            ],
        );

        let agent = AgentLoop::with_tools_and_memory_provider_inner(
            bus,
            provider,
            workspace,
            None,
            Some(1),
            config,
            None,
            file_manager,
            None,
            Some(Some(runtime)),
        )
        .await
        .unwrap();

        assert_eq!(agent.mentle_active(), agent.tools.has("memtle_status"));
        assert!(agent.tools.has("memtle_search"));
        assert!(agent.context.build_system_prompt().contains("memtle_*"));
    }

    #[tokio::test]
    async fn test_with_toolset_memtle_tool_without_status_disables_prompt() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(NamedTool {
            name: "memtle_search",
        }));
        let mut config = ToolConfig::default();
        config.builtin.mentle = true;
        let toolset = AgentLoopToolSet { registry, config };

        let agent = AgentLoop::with_toolset(
            bus,
            provider,
            workspace,
            None,
            Some(1),
            toolset,
            None,
            file_manager,
        )
        .await
        .unwrap();

        let prompt = agent.context.build_system_prompt();

        assert!(!agent.mentle_active());
        assert!(agent.tools.has("memtle_search"));
        assert!(!agent.tools.has("memtle_status"));
        assert!(!prompt.contains("L2 Palace Memory"));
        assert!(!prompt.contains("memtle_search"));
    }

    #[tokio::test]
    async fn test_with_toolset_keeps_external_registry_isolated_from_runtime_state() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(NamedTool {
            name: "memtle_status",
        }));
        let mut config = ToolConfig::default();
        config.builtin.mentle = true;
        config.builtin.spawn = true;
        let toolset = AgentLoopToolSet { registry, config };

        let agent = AgentLoop::with_toolset(
            bus,
            provider,
            workspace,
            None,
            Some(1),
            toolset,
            None,
            file_manager,
        )
        .await
        .unwrap();

        assert!(agent.mentle_active());
        assert!(agent.tools.has("memtle_status"));
        assert!(agent.custom_tools.is_empty());
        assert!(!agent.subagent_manager.builtin_tools_for_test().mentle);
        assert!(!agent.subagent_manager.builtin_tools_for_test().spawn);
        #[cfg(feature = "mentle")]
        assert!(agent.mentle_runtime.is_none());
    }

    #[tokio::test]
    async fn test_build_agent_tools_reuses_custom_tools_with_cron() {
        let temp_dir = tempfile::tempdir().unwrap();
        let mut config = ToolConfig::default();
        config.builtin = BuiltInToolsConfig {
            cron: true,
            ..BuiltInToolsConfig::none()
        };
        let cron_service = Arc::new(CronService::new(temp_dir.path().join("cron.json"), None));
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );

        let registry = build_agent_tools(
            temp_dir.path().to_path_buf(),
            &config,
            Arc::new(NoopSpawner),
            file_manager,
            vec![Arc::new(NamedTool {
                name: "memtle_status",
            })],
            Some(cron_service),
        );

        assert!(registry.has("memtle_status"));
        assert!(registry.has("cron"));
    }

    #[tokio::test]
    async fn test_register_default_tools_preserves_custom_tools_with_cron() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut agent = AgentLoop::with_tools(
            bus,
            provider,
            workspace.clone(),
            None,
            Some(1),
            ToolConfig::default(),
            None,
            file_manager,
        )
        .await
        .unwrap();
        let mut config = ToolConfig::default();
        config.builtin = BuiltInToolsConfig {
            cron: true,
            ..BuiltInToolsConfig::none()
        };
        config.cron_service = Some(Arc::new(CronService::new(
            workspace.join("cron.json"),
            None,
        )));

        agent.custom_tools = vec![Arc::new(NamedTool {
            name: "memtle_status",
        })];
        agent.register_default_tools(config);

        assert!(agent.tools.has("memtle_status"));
        assert!(agent.tools.has("cron"));
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_with_tools_active_runtime_enables_registry_and_prompt() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut config = ToolConfig::default();
        config.builtin.mentle = true;
        config.mentle = MentleToolRuntimeConfig {
            enabled: true,
            mode: MentleToolMode::Full,
            allowed_tools: Vec::new(),
        };
        let toolkit = memtle::toolkit::MemtleToolkit::open(workspace.join("palace.db"))
            .await
            .unwrap();
        let runtime = MentleRuntime::from_parts_for_test(
            Arc::new(tokio::sync::Mutex::new(toolkit)),
            Arc::new(agent_diva_core::memory::MemoryManager::new(&workspace)),
            vec![Arc::new(NamedTool {
                name: "memtle_status",
            })],
        );

        let agent = AgentLoop::with_tools_and_memory_provider_inner(
            bus,
            provider,
            workspace,
            None,
            Some(1),
            config,
            None,
            file_manager,
            None,
            Some(Some(runtime)),
        )
        .await
        .unwrap();

        assert!(agent.mentle_active());
        assert!(agent.tools.has("memtle_status"));
        assert!(agent
            .context
            .build_system_prompt()
            .contains("L2 Palace Memory"));
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_with_tools_startup_cron_preserves_mentle_custom_tools() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut config = ToolConfig::default();
        config.builtin = BuiltInToolsConfig {
            mentle: true,
            cron: true,
            ..BuiltInToolsConfig::none()
        };
        config.mentle = MentleToolRuntimeConfig {
            enabled: true,
            mode: MentleToolMode::Full,
            allowed_tools: Vec::new(),
        };
        config.cron_service = Some(Arc::new(CronService::new(
            workspace.join("cron.json"),
            None,
        )));
        let toolkit = memtle::toolkit::MemtleToolkit::open(workspace.join("palace.db"))
            .await
            .unwrap();
        let runtime = MentleRuntime::from_parts_for_test(
            Arc::new(tokio::sync::Mutex::new(toolkit)),
            Arc::new(agent_diva_core::memory::MemoryManager::new(&workspace)),
            vec![Arc::new(NamedTool {
                name: "memtle_status",
            })],
        );

        let agent = AgentLoop::with_tools_and_memory_provider_inner(
            bus,
            provider,
            workspace,
            None,
            Some(1),
            config,
            None,
            file_manager,
            None,
            Some(Some(runtime)),
        )
        .await
        .unwrap();

        assert!(agent.mentle_active());
        assert!(agent.tools.has("memtle_status"));
        assert!(agent.tools.has("cron"));
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_register_default_tools_rebuild_keeps_active_mentle_prompt() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut config = ToolConfig::default();
        config.builtin.mentle = true;
        config.mentle = MentleToolRuntimeConfig {
            enabled: true,
            mode: MentleToolMode::Full,
            allowed_tools: Vec::new(),
        };
        let toolkit = memtle::toolkit::MemtleToolkit::open(workspace.join("palace.db"))
            .await
            .unwrap();
        let runtime = MentleRuntime::from_parts_for_test(
            Arc::new(tokio::sync::Mutex::new(toolkit)),
            Arc::new(agent_diva_core::memory::MemoryManager::new(&workspace)),
            vec![Arc::new(NamedTool {
                name: "memtle_status",
            })],
        );

        let mut agent = AgentLoop::with_tools_and_memory_provider_inner(
            bus,
            provider,
            workspace.clone(),
            None,
            Some(1),
            config,
            None,
            file_manager,
            None,
            Some(Some(runtime)),
        )
        .await
        .unwrap();
        let mut rebuild_config = ToolConfig::default();
        rebuild_config.builtin = BuiltInToolsConfig {
            mentle: true,
            cron: true,
            ..BuiltInToolsConfig::none()
        };
        rebuild_config.cron_service = Some(Arc::new(CronService::new(
            workspace.join("cron.json"),
            None,
        )));

        agent.register_default_tools(rebuild_config);

        assert!(agent.mentle_active());
        assert!(agent.tools.has("memtle_status"));
        assert!(agent.tools.has("cron"));
        assert!(agent
            .context
            .build_system_prompt()
            .contains("L2 Palace Memory"));
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_tool_definitions_are_dynamic() {
        let temp_dir = tempfile::tempdir().unwrap();
        let toolkit = memtle::toolkit::MemtleToolkit::open(temp_dir.path().join("palace.db"))
            .await
            .unwrap();
        let names = toolkit
            .tool_definitions()
            .into_iter()
            .filter_map(|def| {
                def.get("name")
                    .and_then(|name| name.as_str())
                    .map(str::to_string)
            })
            .collect::<std::collections::HashSet<_>>();

        assert!(names.contains("memtle_status"));
        assert!(names.contains("memtle_search"));
        assert!(names.contains("memtle_diary_write"));
    }

    #[cfg(feature = "mentle")]
    #[test]
    fn test_mentle_tool_definition_metadata_maps_mcp_schema() {
        let definition = serde_json::json!({
            "name": "memtle_search",
            "description": "Search the memory palace",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                },
                "required": ["query"]
            }
        });

        let (name, description, parameters) =
            mentle_tool_metadata_from_definition(&definition).unwrap();

        assert_eq!(name, "memtle_search");
        assert_eq!(description, "Search the memory palace");
        assert_eq!(parameters["required"][0], "query");
    }

    #[cfg(feature = "mentle")]
    #[test]
    fn test_mentle_tool_definition_metadata_rejects_incomplete_schema() {
        let missing_schema = serde_json::json!({
            "name": "memtle_search",
            "description": "Search the memory palace"
        });
        let missing_name = serde_json::json!({
            "description": "Search the memory palace",
            "inputSchema": { "type": "object" }
        });
        let invalid_schema = serde_json::json!({
            "name": "memtle_search",
            "description": "Search the memory palace",
            "inputSchema": "not a json schema object"
        });

        assert!(mentle_tool_metadata_from_definition(&missing_schema).is_none());
        assert!(mentle_tool_metadata_from_definition(&missing_name).is_none());
        assert!(mentle_tool_metadata_from_definition(&invalid_schema).is_none());
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_tools_skip_invalid_definitions() {
        let temp_dir = tempfile::tempdir().unwrap();
        let toolkit = memtle::toolkit::MemtleToolkit::open(temp_dir.path().join("palace.db"))
            .await
            .unwrap();
        let toolkit = Arc::new(tokio::sync::Mutex::new(toolkit));
        let tools = mentle_tools_from_definitions(
            vec![
                serde_json::json!({
                    "name": "memtle_status",
                    "description": "Return palace status",
                    "inputSchema": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }),
                serde_json::json!({
                    "name": "memtle_broken",
                    "description": "Broken schema",
                    "inputSchema": "not an object"
                }),
            ],
            toolkit,
        );

        let names = tools
            .iter()
            .map(|tool| tool.name().to_string())
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(
            names,
            std::collections::HashSet::from(["memtle_status".to_string()])
        );
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_tool_filter_custom_mode_ignores_unknown_names() {
        let temp_dir = tempfile::tempdir().unwrap();
        let toolkit = memtle::toolkit::MemtleToolkit::open(temp_dir.path().join("palace.db"))
            .await
            .unwrap();
        let toolkit = Arc::new(tokio::sync::Mutex::new(toolkit));
        let tools = mentle_tools_from_definitions(
            vec![
                serde_json::json!({
                    "name": "memtle_status",
                    "description": "Return palace status",
                    "inputSchema": { "type": "object", "properties": {}, "required": [] }
                }),
                serde_json::json!({
                    "name": "memtle_search",
                    "description": "Search palace",
                    "inputSchema": { "type": "object", "properties": {}, "required": [] }
                }),
            ],
            toolkit,
        );
        let config = MentleToolRuntimeConfig {
            enabled: true,
            mode: MentleToolMode::Custom,
            allowed_tools: vec!["memtle_status".to_string(), "shell".to_string()],
        };
        let filtered = crate::mentle_runtime::filter_mentle_tools(tools, &config);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name(), "memtle_status");
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_tool_filter_read_only_excludes_mutation_tools() {
        let temp_dir = tempfile::tempdir().unwrap();
        let toolkit = memtle::toolkit::MemtleToolkit::open(temp_dir.path().join("palace.db"))
            .await
            .unwrap();
        let toolkit = Arc::new(tokio::sync::Mutex::new(toolkit));
        let tools = mentle_tools_from_definitions(
            vec![
                serde_json::json!({
                    "name": "memtle_status",
                    "description": "Return palace status",
                    "inputSchema": { "type": "object", "properties": {}, "required": [] }
                }),
                serde_json::json!({
                    "name": "memtle_search",
                    "description": "Search palace",
                    "inputSchema": { "type": "object", "properties": {}, "required": [] }
                }),
                serde_json::json!({
                    "name": "memtle_diary_write",
                    "description": "Write diary",
                    "inputSchema": { "type": "object", "properties": {}, "required": [] }
                }),
            ],
            toolkit,
        );
        let config = MentleToolRuntimeConfig {
            enabled: true,
            mode: MentleToolMode::ReadOnly,
            allowed_tools: Vec::new(),
        };
        let names = crate::mentle_runtime::filter_mentle_tools(tools, &config)
            .into_iter()
            .map(|tool| tool.name().to_string())
            .collect::<std::collections::HashSet<_>>();

        assert!(names.contains("memtle_status"));
        assert!(names.contains("memtle_search"));
        assert!(!names.contains("memtle_diary_write"));
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_tool_adapter_executes_json_call() {
        let temp_dir = tempfile::tempdir().unwrap();
        let toolkit = memtle::toolkit::MemtleToolkit::open(temp_dir.path().join("palace.db"))
            .await
            .unwrap();
        let toolkit = Arc::new(tokio::sync::Mutex::new(toolkit));
        let definition = serde_json::json!({
            "name": "memtle_status",
            "description": "Return palace status",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        });
        let tool = mentle_tool_from_definition(&definition, toolkit).unwrap();
        let schema = tool.to_schema();

        assert_eq!(schema["function"]["name"], "memtle_status");
        assert_eq!(schema["function"]["parameters"], definition["inputSchema"]);

        let output = tool.execute(serde_json::json!({})).await.unwrap();

        assert!(output.contains("total_drawers"));
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_tool_adapter_translates_call_errors() {
        let temp_dir = tempfile::tempdir().unwrap();
        let toolkit = memtle::toolkit::MemtleToolkit::open(temp_dir.path().join("palace.db"))
            .await
            .unwrap();
        let tool = MentleToolkitTool {
            name: "memtle_status".to_string(),
            description: "Return palace status".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            toolkit: Arc::new(tokio::sync::Mutex::new(toolkit)),
        };

        let error = tool
            .execute(serde_json::json!("not an object"))
            .await
            .expect_err("non-object args should be rejected by MemtleToolkit");

        assert!(error
            .to_string()
            .contains("tool arguments must be a JSON object"));
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_tool_adapter_translates_payload_error() {
        let temp_dir = tempfile::tempdir().unwrap();
        let toolkit = memtle::toolkit::MemtleToolkit::open(temp_dir.path().join("palace.db"))
            .await
            .unwrap();
        let tool = MentleToolkitTool {
            name: "memtle_add_drawer".to_string(),
            description: "Add drawer".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            toolkit: Arc::new(tokio::sync::Mutex::new(toolkit)),
        };

        let error = tool
            .execute(serde_json::json!({}))
            .await
            .expect_err("payload error should become a tool execution error");

        assert!(error.to_string().contains("must be a non-empty string"));
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_tool_adapter_translates_unknown_tool_payload() {
        let temp_dir = tempfile::tempdir().unwrap();
        let toolkit = memtle::toolkit::MemtleToolkit::open(temp_dir.path().join("palace.db"))
            .await
            .unwrap();
        let tool = MentleToolkitTool {
            name: "memtle_unknown_tool".to_string(),
            description: "Unknown tool for error mapping".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            toolkit: Arc::new(tokio::sync::Mutex::new(toolkit)),
        };

        let error = tool
            .execute(serde_json::json!({}))
            .await
            .expect_err("unknown tool payload should become a tool execution error");

        assert!(error.to_string().contains("Unknown tool"));
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_tool_registration_skips_invalid_definition_only() {
        let temp_dir = tempfile::tempdir().unwrap();
        let toolkit = memtle::toolkit::MemtleToolkit::open(temp_dir.path().join("palace.db"))
            .await
            .unwrap();
        let toolkit = Arc::new(tokio::sync::Mutex::new(toolkit));
        let definitions = vec![
            serde_json::json!({
                "description": "Missing name",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
            serde_json::json!({
                "name": "memtle_status",
                "description": "Return palace status",
                "inputSchema": {
                    "type": "object",
                    "properties": {},
                    "required": []
                }
            }),
        ];

        let tools = mentle_tools_from_definitions(definitions, toolkit);

        let names = tools
            .iter()
            .map(|tool| tool.name().to_string())
            .collect::<std::collections::HashSet<_>>();

        assert_eq!(
            names,
            std::collections::HashSet::from(["memtle_status".to_string()])
        );
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_open_failure_falls_back_to_markdown_memory() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut config = ToolConfig::default();
        config.builtin.mentle = true;
        config.mentle = MentleToolRuntimeConfig {
            enabled: true,
            mode: MentleToolMode::Full,
            allowed_tools: Vec::new(),
        };
        std::fs::create_dir_all(workspace.join("memory")).unwrap();
        std::fs::write(
            workspace.join("memory").join("MEMORY.md"),
            "Markdown fallback continuity.",
        )
        .unwrap();

        let agent = AgentLoop::with_tools_and_memory_provider_inner(
            bus,
            provider,
            workspace,
            None,
            Some(1),
            config,
            None,
            file_manager,
            None,
            Some(None),
        )
        .await
        .unwrap();

        let prompt = agent.context.build_system_prompt();

        assert!(!agent.mentle_active());
        assert!(!agent.tools.has("memtle_status"));
        assert!(!prompt.contains("L2 Palace Memory"));
        assert!(!prompt.contains("memtle_search"));
        assert!(prompt.contains("Markdown fallback continuity."));
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_memory_dir_create_failure_disables_runtime() {
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        std::fs::write(workspace.join("memory"), "not a directory").unwrap();

        let runtime = MentleRuntime::try_build(
            &workspace,
            &crate::tool_config::mentle::MentleToolRuntimeConfig {
                enabled: true,
                mode: crate::tool_config::mentle::MentleToolMode::Full,
                allowed_tools: Vec::new(),
            },
        )
        .await;

        assert!(runtime.is_none());
    }

    #[cfg(feature = "mentle")]
    #[tokio::test]
    async fn test_mentle_runtime_without_status_disables_prompt_routing() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let mut config = ToolConfig::default();
        config.builtin.mentle = true;
        config.mentle = MentleToolRuntimeConfig {
            enabled: true,
            mode: MentleToolMode::Custom,
            allowed_tools: vec!["memtle_search".to_string()],
        };
        let toolkit = memtle::toolkit::MemtleToolkit::open(workspace.join("palace.db"))
            .await
            .unwrap();
        let runtime = MentleRuntime::from_parts_for_test(
            Arc::new(tokio::sync::Mutex::new(toolkit)),
            Arc::new(agent_diva_core::memory::MemoryManager::new(&workspace)),
            vec![Arc::new(NamedTool {
                name: "memtle_search",
            })],
        );

        let agent = AgentLoop::with_tools_and_memory_provider_inner(
            bus,
            provider,
            workspace,
            None,
            Some(1),
            config,
            None,
            file_manager,
            None,
            Some(Some(runtime)),
        )
        .await
        .unwrap();

        let prompt = agent.context.build_system_prompt();

        assert!(!agent.mentle_active());
        assert!(agent.tools.has("memtle_search"));
        assert!(!agent.tools.has("memtle_status"));
        assert!(!prompt.contains("L2 Palace Memory"));
        assert!(!prompt.contains("memtle_search"));
    }

    use agent_diva_core::memory::{
        PrefetchRequest, PrefetchResponse, PrefetchStatus, SessionEndRequest, SessionEndResponse,
        SessionEndStatus, StartupStatus, SyncTurnRequest, SyncTurnResponse, SyncTurnStatus,
        SystemPromptBlock, SystemPromptRequest, SystemPromptResponse,
    };
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    /// A test memory provider that tracks which lifecycle hooks were called.
    struct TrackingMemoryProvider {
        startup_called: AtomicBool,
        prefetch_called: AtomicBool,
        sync_called: AtomicBool,
        session_end_called: AtomicBool,
        prefetch_count: AtomicUsize,
        sync_count: AtomicUsize,
        prefetch_failure_reason: Option<String>,
        sync_failure_reason: Option<String>,
    }

    impl TrackingMemoryProvider {
        fn new() -> Self {
            Self {
                startup_called: AtomicBool::new(false),
                prefetch_called: AtomicBool::new(false),
                sync_called: AtomicBool::new(false),
                session_end_called: AtomicBool::new(false),
                prefetch_count: AtomicUsize::new(0),
                sync_count: AtomicUsize::new(0),
                prefetch_failure_reason: None,
                sync_failure_reason: None,
            }
        }

        fn with_prefetch_failure(reason: impl Into<String>) -> Self {
            Self {
                prefetch_failure_reason: Some(reason.into()),
                ..Self::new()
            }
        }

        fn with_sync_failure(reason: impl Into<String>) -> Self {
            Self {
                sync_failure_reason: Some(reason.into()),
                ..Self::new()
            }
        }
    }

    #[async_trait::async_trait]
    impl MemoryProvider for TrackingMemoryProvider {
        fn system_prompt_block(
            &self,
            _request: &SystemPromptRequest,
        ) -> agent_diva_core::Result<SystemPromptResponse> {
            self.startup_called.store(true, Ordering::SeqCst);
            Ok(SystemPromptResponse::ready(SystemPromptBlock {
                shape: agent_diva_core::memory::StartupInjectionShape::CompactRenderedMarkdown,
                markdown: "## Tracking Provider Startup\nTest continuity injected.".to_string(),
            }))
        }

        async fn prefetch(
            &self,
            request: PrefetchRequest,
        ) -> agent_diva_core::Result<PrefetchResponse> {
            self.prefetch_called.store(true, Ordering::SeqCst);
            self.prefetch_count.fetch_add(1, Ordering::SeqCst);

            if request.intent.trim().is_empty() {
                return Ok(PrefetchResponse {
                    status: PrefetchStatus::SkippedNoIntent,
                    prompt_block: None,
                });
            }

            if let Some(reason) = &self.prefetch_failure_reason {
                return Ok(PrefetchResponse {
                    status: PrefetchStatus::Failed {
                        reason: reason.clone(),
                    },
                    prompt_block: None,
                });
            }

            Ok(PrefetchResponse {
                status: PrefetchStatus::Ready,
                prompt_block: Some(format!("## Prefetch Recall\nIntent: {}", request.intent)),
            })
        }

        async fn sync_turn(
            &self,
            _request: SyncTurnRequest,
        ) -> agent_diva_core::Result<SyncTurnResponse> {
            self.sync_called.store(true, Ordering::SeqCst);
            self.sync_count.fetch_add(1, Ordering::SeqCst);
            if let Some(reason) = &self.sync_failure_reason {
                return Ok(SyncTurnResponse {
                    status: SyncTurnStatus::Failed {
                        reason: reason.clone(),
                    },
                });
            }

            Ok(SyncTurnResponse {
                status: SyncTurnStatus::Persisted,
            })
        }

        async fn on_session_end(
            &self,
            _request: SessionEndRequest,
        ) -> agent_diva_core::Result<SessionEndResponse> {
            self.session_end_called.store(true, Ordering::SeqCst);
            Ok(SessionEndResponse {
                status: SessionEndStatus::Triggered,
            })
        }
    }

    #[tokio::test]
    async fn test_agent_loop_accepts_custom_memory_provider() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let memory_provider = Arc::new(TrackingMemoryProvider::new());

        let _agent = AgentLoop::with_tools_and_memory_provider(
            bus,
            provider.clone(),
            workspace,
            None,
            Some(1),
            ToolConfig::default(),
            None,
            Arc::new(
                agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                    &temp_dir.path().join("files"),
                ))
                .await
                .unwrap(),
            ),
            Some(memory_provider.clone()),
        )
        .await
        .unwrap();

        // Verify the provider is the one we injected (Arc pointer identity).
        assert_eq!(Arc::strong_count(&memory_provider), 3); // agent, context, and test handle
    }

    #[tokio::test]
    async fn test_prefetch_recall_block_is_injected_before_first_llm_call() {
        let bus = MessageBus::new();
        let provider = Arc::new(CapturingStreamProvider::default());
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let memory_provider = Arc::new(TrackingMemoryProvider::new());
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );

        let mut agent = AgentLoop::with_tools_and_memory_provider(
            bus,
            provider.clone(),
            workspace,
            None,
            Some(1),
            ToolConfig::default(),
            None,
            file_manager,
            Some(memory_provider.clone()),
        )
        .await
        .unwrap();

        let response = agent
            .process_direct("recall provider boundary", "session-1", "gui", "chat-1")
            .await
            .unwrap();

        assert_eq!(response, "done");
        assert_eq!(memory_provider.prefetch_count.load(Ordering::SeqCst), 1);

        let captured = provider.captured_messages.lock().unwrap();
        let first_call = captured
            .first()
            .expect("provider should capture the first LLM call");
        assert!(first_call.len() >= 3);
        assert_eq!(first_call[0].role, "system");
        assert_eq!(first_call[1].role, "system");
        assert!(first_call[1].content.contains("## Prefetch Recall"));
        assert!(first_call[1].content.contains("recall provider boundary"));
        assert_eq!(first_call[2].role, "user");
        assert_eq!(first_call[2].content, "recall provider boundary");
    }

    #[tokio::test]
    async fn test_agent_loop_prefetch_failure_continues_without_recall_injection() {
        let bus = MessageBus::new();
        let provider = Arc::new(CapturingStreamProvider::default());
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let memory_provider = Arc::new(TrackingMemoryProvider::with_prefetch_failure(
            "palace query unavailable",
        ));

        let mut agent = AgentLoop::with_tools_and_memory_provider(
            bus,
            provider.clone(),
            workspace,
            None,
            Some(1),
            ToolConfig::default(),
            None,
            file_manager,
            Some(memory_provider.clone()),
        )
        .await
        .unwrap();

        let response = agent
            .process_direct("recall provider boundary", "session-1", "gui", "chat-1")
            .await
            .unwrap();

        assert_eq!(response, "done");
        assert_eq!(memory_provider.prefetch_count.load(Ordering::SeqCst), 1);

        let captured = provider.captured_messages.lock().unwrap();
        let first_call = captured
            .first()
            .expect("provider should capture the first LLM call");
        assert!(first_call
            .iter()
            .all(|message| !message.content.contains("## Prefetch Recall")));
        assert_eq!(
            first_call
                .last()
                .expect("first call should include a user message")
                .content,
            "recall provider boundary"
        );
    }

    #[tokio::test]
    async fn test_agent_loop_consolidation_sync_failure_keeps_main_response() {
        let bus = MessageBus::new();
        let provider = Arc::new(ConsolidatingStreamProvider::default());
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            agent_diva_files::FileManager::new(agent_diva_files::FileConfig::with_path(
                &temp_dir.path().join("files"),
            ))
            .await
            .unwrap(),
        );
        let memory_provider = Arc::new(TrackingMemoryProvider::with_sync_failure(
            "palace write unavailable",
        ));

        let mut agent = AgentLoop::with_tools_and_memory_provider(
            bus,
            provider.clone(),
            workspace,
            None,
            Some(1),
            ToolConfig::default(),
            None,
            file_manager,
            Some(memory_provider.clone()),
        )
        .await
        .unwrap();
        agent.memory_window = 1;

        let response = agent
            .process_direct("hello", "session-1", "gui", "chat-1")
            .await
            .unwrap();

        assert_eq!(response, "done");
        assert_eq!(memory_provider.sync_count.load(Ordering::SeqCst), 1);
        let captured = provider.captured_messages.lock().unwrap();
        assert_eq!(captured.len(), 1);
    }

    #[tokio::test]
    async fn test_memory_provider_startup_called_during_context_build() {
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let memory_provider = Arc::new(TrackingMemoryProvider::new());

        // ContextBuilder with injected provider
        let builder = ContextBuilder::new(workspace).with_memory_provider(memory_provider.clone());

        let prompt = builder.build_system_prompt();

        // Startup hook should have been called synchronously.
        assert!(
            memory_provider.startup_called.load(Ordering::SeqCst),
            "system_prompt_block should be called during build_system_prompt"
        );
        assert!(
            prompt.contains("Test continuity injected"),
            "startup continuity should appear in the system prompt"
        );
    }

    #[tokio::test]
    async fn test_memory_provider_prefetch_skips_on_blank_intent() {
        let provider = TrackingMemoryProvider::new();

        let response = provider
            .prefetch(PrefetchRequest {
                workspace_root: PathBuf::from("/tmp"),
                intent: "   ".to_string(),
                current_room: None,
                user_message: Some("help".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(response.status, PrefetchStatus::SkippedNoIntent);
        assert!(response.prompt_block.is_none());
        assert!(
            provider.prefetch_called.load(Ordering::SeqCst),
            "prefetch should be called even when intent is blank"
        );
    }

    #[tokio::test]
    async fn test_memory_provider_prefetch_runs_on_valid_intent() {
        let provider = TrackingMemoryProvider::new();

        let response = provider
            .prefetch(PrefetchRequest {
                workspace_root: PathBuf::from("/tmp"),
                intent: "recall provider boundary".to_string(),
                current_room: Some("roadmap".to_string()),
                user_message: Some("status?".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(response.status, PrefetchStatus::Ready);
        assert!(response.prompt_block.is_some());
        assert!(response
            .prompt_block
            .unwrap()
            .contains("recall provider boundary"));
    }

    #[tokio::test]
    async fn test_memory_provider_sync_turn_records_call() {
        let provider = TrackingMemoryProvider::new();

        let response = provider
            .sync_turn(SyncTurnRequest {
                workspace_root: PathBuf::from("/tmp"),
                memory_update_markdown: Some("Updated memory".to_string()),
                history_entry: Some("task complete".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(response.status, SyncTurnStatus::Persisted);
        assert!(provider.sync_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_memory_provider_session_end_called_at_shutdown() {
        let provider = TrackingMemoryProvider::new();

        let response = provider
            .on_session_end(SessionEndRequest {
                workspace_root: PathBuf::from("/tmp"),
                session_id: Some("test-session".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(response.status, SessionEndStatus::Triggered);
        assert!(provider.session_end_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn test_memory_provider_full_lifecycle_hooks() {
        let provider = TrackingMemoryProvider::new();

        // 1. Startup
        let startup = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: PathBuf::from("/tmp"),
            })
            .unwrap();
        assert!(matches!(startup.status, StartupStatus::Ready));
        assert!(provider.startup_called.load(Ordering::SeqCst));

        // 2. Prefetch with intent
        let _prefetch = provider
            .prefetch(PrefetchRequest {
                workspace_root: PathBuf::from("/tmp"),
                intent: "review memory".to_string(),
                current_room: None,
                user_message: None,
            })
            .await
            .unwrap();
        assert!(provider.prefetch_called.load(Ordering::SeqCst));

        // 3. Sync after successful turn
        let sync = provider
            .sync_turn(SyncTurnRequest {
                workspace_root: PathBuf::from("/tmp"),
                memory_update_markdown: Some("evidence".to_string()),
                history_entry: None,
            })
            .await
            .unwrap();
        assert_eq!(sync.status, SyncTurnStatus::Persisted);
        assert!(provider.sync_called.load(Ordering::SeqCst));

        // 4. Session end
        let shutdown = provider
            .on_session_end(SessionEndRequest {
                workspace_root: PathBuf::from("/tmp"),
                session_id: Some("lifecycle".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(shutdown.status, SessionEndStatus::Triggered);
        assert!(provider.session_end_called.load(Ordering::SeqCst));
    }
}
