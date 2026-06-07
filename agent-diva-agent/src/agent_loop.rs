//! Agent loop: the core processing engine

use agent_diva_core::bus::{AgentEvent, InboundMessage, MessageBus, OutboundMessage};
use agent_diva_core::config::MCPServerConfig;
use agent_diva_core::cron::CronService;
use agent_diva_core::debug::DebugEventLogger;
use agent_diva_core::error_context::ErrorContext;
use agent_diva_core::memory::{MemoryProvider, SessionEndRequest};
use agent_diva_core::session::SessionManager;
use agent_diva_core::trace::{TraceId, TraceLogger};
use agent_diva_files::{FileConfig, FileManager};
use agent_diva_providers::LLMProvider;
use agent_diva_tooling::{ToolError, ToolRegistry};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};

use crate::consolidation;
use crate::context::{ContextBuilder, SoulContextSettings};
use crate::context_budget::ContextBudgetPolicy;
use crate::runtime_control::RuntimeControlCommand;
use crate::subagent::SubagentManager;
use crate::subagent::SubagentSpawnRequest;
use crate::subagent_policy::SubagentPolicy;
use crate::tool_assembly::{SubagentSpawner, ToolAssembly};
use crate::tool_config::builtin::BuiltInToolsConfig;
use crate::tool_config::network::NetworkToolConfig;

pub(crate) mod context_retry;
mod loop_guard;
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
    /// Default tool execution timeout in seconds
    pub exec_timeout: u64,
    /// Whether to restrict file access to workspace
    pub restrict_to_workspace: bool,
    /// Configured MCP servers
    pub mcp_servers: HashMap<String, MCPServerConfig>,
    /// Subagent delegation policy
    pub subagent_policy: SubagentPolicy,
    /// Optional cron service for scheduling tools
    pub cron_service: Option<Arc<CronService>>,
    /// Soul context settings
    pub soul_context: SoulContextSettings,
    /// Response/request runtime settings
    pub request_max_tokens: i32,
    pub temperature: f64,
    pub context_budget: ContextBudgetPolicy,
    /// Structured runtime observability logger.
    pub trace_logger: Option<Arc<TraceLogger>>,
    /// Explicit raw debug logger for foreground gateway debug runs.
    pub debug_logger: Option<Arc<DebugEventLogger>>,
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
            exec_timeout: 60,
            restrict_to_workspace: false,
            mcp_servers: HashMap::new(),
            subagent_policy: SubagentPolicy::default(),
            cron_service: None,
            soul_context: SoulContextSettings::default(),
            request_max_tokens: 4096,
            temperature: 0.7,
            context_budget: ContextBudgetPolicy::default(),
            trace_logger: None,
            debug_logger: None,
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
    request_max_tokens: i32,
    temperature: f64,
    context_budget: ContextBudgetPolicy,
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
    trace_logger: Option<Arc<TraceLogger>>,
    debug_logger: Option<Arc<DebugEventLogger>>,
}

pub struct AgentLoopToolSet {
    pub registry: ToolRegistry,
    pub config: ToolConfig,
}

struct SubagentManagerSpawner {
    manager: Arc<SubagentManager>,
}

#[async_trait::async_trait]
impl SubagentSpawner for SubagentManagerSpawner {
    async fn spawn(&self, request: SubagentSpawnRequest) -> Result<String, ToolError> {
        self.manager
            .spawn(request)
            .await
            .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
    }
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
            BuiltInToolsConfig::default(),
            NetworkToolConfig::default(),
            None,
            false,
            HashMap::new(),
            SubagentPolicy::default(),
            ToolConfig::default().context_budget,
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
            request_max_tokens: ToolConfig::default().request_max_tokens,
            temperature: ToolConfig::default().temperature,
            context_budget: ToolConfig::default().context_budget,
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
            trace_logger: None,
            debug_logger: None,
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
        let model = model.unwrap_or_else(|| provider.get_default_model());
        let mut context = ContextBuilder::with_skills(workspace.clone(), None);
        context.set_soul_settings(tool_config.soul_context.clone());
        let sessions = SessionManager::new(workspace.clone());

        let subagent_manager = Arc::new(SubagentManager::new(
            provider.clone(),
            workspace.clone(),
            bus.clone(),
            Some(model.clone()),
            tool_config.builtin.clone(),
            tool_config.network.clone(),
            Some(tool_config.exec_timeout),
            tool_config.restrict_to_workspace,
            tool_config.mcp_servers.clone(),
            tool_config.subagent_policy.clone(),
            tool_config.context_budget.clone(),
        ));

        let spawner = Arc::new(SubagentManagerSpawner {
            manager: subagent_manager.clone(),
        });
        let tools = ToolAssembly::new(workspace.clone())
            .builtin(tool_config.builtin.clone())
            .with_network_config(tool_config.network.clone())
            .with_exec_timeout(tool_config.exec_timeout)
            .restrict_to_workspace(tool_config.restrict_to_workspace)
            .mcp_servers(tool_config.mcp_servers.clone())
            .with_subagent_spawner(spawner)
            .with_file_manager(file_manager.clone())
            .with_tools(Vec::new())
            .build();

        let memory_provider = memory_provider
            .unwrap_or_else(|| Arc::new(agent_diva_core::memory::MemoryManager::new(&workspace)));

        let mut agent = Self {
            bus,
            provider,
            workspace,
            model,
            request_max_tokens: tool_config.request_max_tokens,
            temperature: tool_config.temperature,
            context_budget: tool_config.context_budget.clone(),
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
            trace_logger: tool_config.trace_logger.clone(),
            debug_logger: tool_config.debug_logger.clone(),
        };

        if let Some(cron_service) = agent.tool_config.cron_service.clone() {
            agent.tools = ToolAssembly::new(agent.workspace.clone())
                .builtin(agent.tool_config.builtin.clone())
                .with_network_config(agent.tool_config.network.clone())
                .with_exec_timeout(agent.tool_config.exec_timeout)
                .restrict_to_workspace(agent.tool_config.restrict_to_workspace)
                .mcp_servers(agent.tool_config.mcp_servers.clone())
                .with_subagent_spawner(Arc::new(SubagentManagerSpawner {
                    manager: agent.subagent_manager.clone(),
                }))
                .with_cron_service(cron_service)
                .with_file_manager(agent.file_manager.clone())
                .build();
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
        let sessions = SessionManager::new(workspace.clone());
        let subagent_manager = Arc::new(SubagentManager::new(
            provider.clone(),
            workspace.clone(),
            bus.clone(),
            Some(model.clone()),
            toolset.config.builtin.clone(),
            toolset.config.network.clone(),
            Some(toolset.config.exec_timeout),
            toolset.config.restrict_to_workspace,
            toolset.config.mcp_servers.clone(),
            toolset.config.subagent_policy.clone(),
            toolset.config.context_budget.clone(),
        ));

        let memory_provider: Arc<dyn MemoryProvider> =
            Arc::new(agent_diva_core::memory::MemoryManager::new(&workspace));

        Ok(Self {
            bus,
            provider,
            workspace,
            model,
            request_max_tokens: toolset.config.request_max_tokens,
            temperature: toolset.config.temperature,
            context_budget: toolset.config.context_budget.clone(),
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
            trace_logger: toolset.config.trace_logger.clone(),
            debug_logger: toolset.config.debug_logger.clone(),
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
        mut msg: InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
    ) -> Result<Option<OutboundMessage>, Box<dyn std::error::Error>> {
        let trace_id = msg
            .metadata
            .get("trace_id")
            .and_then(|value| value.as_str())
            .map(TraceId::from)
            .unwrap_or_default();
        msg.metadata.insert(
            "trace_id".to_string(),
            serde_json::Value::String(trace_id.as_str().to_string()),
        );
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
    use agent_diva_core::trace::TraceLogger;
    use agent_diva_providers::{
        LLMResponse, LLMStreamEvent, LiteLLMClient, Message, ProviderError, ProviderEventStream,
        ProviderResult, ToolCallRequest,
    };
    use agent_diva_tooling::Tool;
    use async_trait::async_trait;
    use chrono::Local;
    use futures::stream;
    use serde_json::json;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;
    use tokio::time::{timeout, Duration};

    struct FailingStreamProvider;
    struct SuccessfulStreamProvider;
    struct OverflowRetryProvider {
        calls: AtomicUsize,
        fail_times: usize,
    }
    struct RepeatingToolStreamProvider {
        args_sequence: Mutex<Vec<HashMap<String, serde_json::Value>>>,
    }
    struct AlwaysFailTool;
    struct ToolThenFinalProvider {
        calls: AtomicUsize,
    }
    struct OkTool;

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

    #[async_trait]
    impl LLMProvider for SuccessfulStreamProvider {
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
            Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
                LLMResponse {
                    content: Some("assistant ok".to_string()),
                    tool_calls: Vec::new(),
                    finish_reason: "stop".to_string(),
                    usage: std::collections::HashMap::new(),
                    reasoning_content: None,
                },
            ))])))
        }

        fn get_default_model(&self) -> String {
            "test-model".to_string()
        }
    }

    #[async_trait]
    impl LLMProvider for OverflowRetryProvider {
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
            let call_index = self.calls.fetch_add(1, Ordering::SeqCst);
            if call_index < self.fail_times {
                return Err(ProviderError::ApiError(
                    "This model's maximum context length is 8192 tokens, however you requested 12000 tokens".to_string(),
                ));
            }

            Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
                LLMResponse {
                    content: Some("assistant recovered".to_string()),
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

    #[async_trait]
    impl LLMProvider for RepeatingToolStreamProvider {
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
            let mut args_sequence = self.args_sequence.lock().unwrap();
            let arguments = args_sequence.remove(0);
            Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
                LLMResponse {
                    content: Some("tool attempt".to_string()),
                    tool_calls: vec![ToolCallRequest {
                        id: "call-1".to_string(),
                        call_type: "function".to_string(),
                        name: "fail_tool".to_string(),
                        arguments,
                    }],
                    finish_reason: "tool_calls".to_string(),
                    usage: HashMap::new(),
                    reasoning_content: None,
                },
            ))])))
        }

        fn get_default_model(&self) -> String {
            "test-model".to_string()
        }
    }

    #[async_trait]
    impl Tool for AlwaysFailTool {
        fn name(&self) -> &str {
            "fail_tool"
        }

        fn description(&self) -> &str {
            "Always returns an error"
        }

        fn parameters(&self) -> serde_json::Value {
            json!({
                "type": "object",
                "properties": {
                    "attempt": { "type": "integer" }
                },
                "required": ["attempt"]
            })
        }

        async fn execute(&self, _args: serde_json::Value) -> agent_diva_tooling::Result<String> {
            Ok("Error: simulated tool failure".to_string())
        }
    }

    #[async_trait]
    impl LLMProvider for ToolThenFinalProvider {
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
            let call_index = self.calls.fetch_add(1, Ordering::SeqCst);
            let response = if call_index == 0 {
                LLMResponse {
                    content: Some("using tool".to_string()),
                    tool_calls: vec![ToolCallRequest {
                        id: "call-ok".to_string(),
                        call_type: "function".to_string(),
                        name: "ok_tool".to_string(),
                        arguments: HashMap::from([("path".to_string(), json!("README.md"))]),
                    }],
                    finish_reason: "tool_calls".to_string(),
                    usage: HashMap::new(),
                    reasoning_content: None,
                }
            } else {
                LLMResponse {
                    content: Some("assistant after tool".to_string()),
                    tool_calls: Vec::new(),
                    finish_reason: "stop".to_string(),
                    usage: HashMap::new(),
                    reasoning_content: None,
                }
            };

            Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
                response,
            ))])))
        }

        fn get_default_model(&self) -> String {
            "test-model".to_string()
        }
    }

    #[async_trait]
    impl Tool for OkTool {
        fn name(&self) -> &str {
            "ok_tool"
        }

        fn description(&self) -> &str {
            "Returns a deterministic success result"
        }

        fn parameters(&self) -> serde_json::Value {
            json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string" }
                },
                "required": ["path"]
            })
        }

        async fn execute(&self, _args: serde_json::Value) -> agent_diva_tooling::Result<String> {
            Ok("completed with ghp_demo token".to_string())
        }
    }

    fn build_trace_logger(temp_dir: &tempfile::TempDir) -> Arc<TraceLogger> {
        Arc::new(TraceLogger::new(
            true,
            temp_dir.path().join("runtime-logs"),
            7,
            280,
            64,
            true,
        ))
    }

    fn trace_log_path(temp_dir: &tempfile::TempDir) -> PathBuf {
        temp_dir
            .path()
            .join("runtime-logs")
            .join(format!("runtime-{}.jsonl", Local::now().format("%Y-%m-%d")))
    }

    fn read_trace_events(temp_dir: &tempfile::TempDir) -> Vec<Value> {
        std::fs::read_to_string(trace_log_path(temp_dir))
            .unwrap()
            .lines()
            .map(|line| serde_json::from_str::<Value>(line).unwrap())
            .collect()
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
    async fn test_process_inbound_persists_user_message_on_provider_failure() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut agent = AgentLoop::new(bus, provider, workspace, None, Some(1))
            .await
            .unwrap();
        let msg = InboundMessage::new("gui", "user", "chat-1", "Hello durable");

        let result = agent.process_inbound_message(msg, None).await;
        assert!(result.is_err());

        let session = agent
            .sessions
            .get_or_load("gui:chat-1")
            .unwrap()
            .cloned()
            .expect("session should persist after provider failure");
        assert_eq!(session.messages.len(), 1);
        assert_eq!(session.messages[0].role, "user");
        assert_eq!(session.messages[0].content, "Hello durable");
    }

    #[tokio::test]
    async fn test_process_inbound_success_does_not_duplicate_user_message() {
        let bus = MessageBus::new();
        let provider = Arc::new(SuccessfulStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut agent = AgentLoop::new(bus, provider, workspace, None, Some(1))
            .await
            .unwrap();
        let msg = InboundMessage::new("gui", "user", "chat-1", "Hello once");

        let response = agent.process_inbound_message(msg, None).await.unwrap();
        assert_eq!(response.unwrap().content, "assistant ok");

        let session = agent
            .sessions
            .get_or_load("gui:chat-1")
            .unwrap()
            .cloned()
            .expect("session should exist after successful turn");
        assert_eq!(session.messages.len(), 2);
        assert_eq!(
            session
                .messages
                .iter()
                .filter(|message| message.role == "user")
                .count(),
            1
        );
        assert_eq!(session.messages[0].content, "Hello once");
        assert_eq!(session.messages[1].content, "assistant ok");
    }

    #[tokio::test]
    async fn test_process_inbound_retries_once_after_context_overflow() {
        let bus = MessageBus::new();
        let provider = Arc::new(OverflowRetryProvider {
            calls: AtomicUsize::new(0),
            fail_times: 1,
        });
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut agent = AgentLoop::new(bus, provider.clone(), workspace, None, Some(1))
            .await
            .unwrap();
        let response = agent
            .process_inbound_message(InboundMessage::new("gui", "user", "chat-1", "Hello"), None)
            .await
            .unwrap()
            .expect("response should exist");

        assert_eq!(response.content, "assistant recovered");
        assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_process_inbound_returns_explicit_message_after_repeated_context_overflow() {
        let bus = MessageBus::new();
        let provider = Arc::new(OverflowRetryProvider {
            calls: AtomicUsize::new(0),
            fail_times: 2,
        });
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();

        let mut agent = AgentLoop::new(bus, provider.clone(), workspace, None, Some(1))
            .await
            .unwrap();
        let response = agent
            .process_inbound_message(InboundMessage::new("gui", "user", "chat-1", "Hello"), None)
            .await
            .unwrap()
            .expect("response should exist");

        assert!(response.content.contains("context is too large"));
        assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    }

    use agent_diva_core::memory::{
        PrefetchRequest, PrefetchResponse, PrefetchStatus, SessionEndRequest, SessionEndResponse,
        SessionEndStatus, StartupStatus, SyncTurnRequest, SyncTurnResponse, SyncTurnStatus,
        SystemPromptBlock, SystemPromptRequest, SystemPromptResponse,
    };
    use std::sync::atomic::AtomicBool;

    /// A test memory provider that tracks which lifecycle hooks were called.
    struct TrackingMemoryProvider {
        startup_called: AtomicBool,
        prefetch_called: AtomicBool,
        sync_called: AtomicBool,
        session_end_called: AtomicBool,
        prefetch_count: AtomicUsize,
        sync_count: AtomicUsize,
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
        assert_eq!(Arc::strong_count(&memory_provider), 2); // one in agent, one here
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

    #[tokio::test]
    async fn test_process_inbound_stops_on_repeated_failed_tool_call() {
        let bus = MessageBus::new();
        let provider = Arc::new(RepeatingToolStreamProvider {
            args_sequence: Mutex::new(vec![
                HashMap::from([("attempt".to_string(), json!(1))]),
                HashMap::from([("attempt".to_string(), json!(1))]),
                HashMap::from([("attempt".to_string(), json!(1))]),
            ]),
        });
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            FileManager::new(FileConfig::with_path(&temp_dir.path().join("files")))
                .await
                .unwrap(),
        );

        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(AlwaysFailTool));
        let toolset = AgentLoopToolSet {
            registry,
            config: ToolConfig::default(),
        };

        let mut agent = AgentLoop::with_toolset(
            bus,
            provider,
            workspace,
            None,
            Some(10),
            toolset,
            None,
            file_manager,
        )
        .await
        .unwrap();

        let response = agent
            .process_inbound_message(InboundMessage::new("gui", "user", "chat-1", "hello"), None)
            .await
            .unwrap()
            .expect("response should exist");

        assert!(response.content.contains("repeated failures"));
        assert!(response.content.contains("fail_tool"));
    }

    #[tokio::test]
    async fn test_process_inbound_does_not_trip_repeated_failure_on_different_args() {
        let bus = MessageBus::new();
        let provider = Arc::new(RepeatingToolStreamProvider {
            args_sequence: Mutex::new(vec![
                HashMap::from([("attempt".to_string(), json!(1))]),
                HashMap::from([("attempt".to_string(), json!(2))]),
            ]),
        });
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            FileManager::new(FileConfig::with_path(&temp_dir.path().join("files")))
                .await
                .unwrap(),
        );

        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(AlwaysFailTool));
        let toolset = AgentLoopToolSet {
            registry,
            config: ToolConfig::default(),
        };

        let mut agent = AgentLoop::with_toolset(
            bus,
            provider,
            workspace,
            None,
            Some(2),
            toolset,
            None,
            file_manager,
        )
        .await
        .unwrap();

        let response = agent
            .process_inbound_message(InboundMessage::new("gui", "user", "chat-1", "hello"), None)
            .await
            .unwrap()
            .expect("response should exist");

        assert!(response.content.contains("maximum tool iterations"));
        assert!(!response.content.contains("repeated failures"));
    }

    #[tokio::test]
    async fn test_structured_runtime_logs_capture_message_and_tool_success() {
        let bus = MessageBus::new();
        let provider = Arc::new(ToolThenFinalProvider {
            calls: AtomicUsize::new(0),
        });
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            FileManager::new(FileConfig::with_path(&temp_dir.path().join("files")))
                .await
                .unwrap(),
        );

        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(OkTool));
        let mut tool_config = ToolConfig::default();
        tool_config.trace_logger = Some(build_trace_logger(&temp_dir));
        let toolset = AgentLoopToolSet {
            registry,
            config: tool_config,
        };

        let mut agent = AgentLoop::with_toolset(
            bus,
            provider,
            workspace,
            None,
            Some(3),
            toolset,
            None,
            file_manager,
        )
        .await
        .unwrap();

        let response = agent
            .process_inbound_message(InboundMessage::new("cli", "user", "chat-1", "hello"), None)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(response.content, "assistant after tool");

        let events = read_trace_events(&temp_dir);
        let names: Vec<_> = events
            .iter()
            .map(|event| event["event"].as_str().unwrap().to_string())
            .collect();
        assert!(names.contains(&"message_received".to_string()));
        assert!(names.contains(&"llm_request_started".to_string()));
        assert!(names.contains(&"llm_response_completed".to_string()));
        assert!(names.contains(&"tool_call_started".to_string()));
        assert!(names.contains(&"tool_call_completed".to_string()));

        let first_trace_id = events[0]["trace_id"].as_str().unwrap().to_string();
        assert!(events
            .iter()
            .all(|event| event["trace_id"].as_str() == Some(first_trace_id.as_str())));
        let tool_completed = events
            .iter()
            .find(|event| event["event"] == "tool_call_completed")
            .unwrap();
        assert_eq!(tool_completed["metadata"]["status"], "ok");
        assert_eq!(tool_completed["metadata"]["tool"], "ok_tool");
        assert!(tool_completed["metadata"]["result_summary"]
            .as_str()
            .unwrap()
            .contains("***REDACTED***"));
    }

    #[tokio::test]
    async fn test_structured_runtime_logs_capture_tool_failure() {
        let bus = MessageBus::new();
        let provider = Arc::new(RepeatingToolStreamProvider {
            args_sequence: Mutex::new(vec![HashMap::from([("attempt".to_string(), json!(1))])]),
        });
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            FileManager::new(FileConfig::with_path(&temp_dir.path().join("files")))
                .await
                .unwrap(),
        );

        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(AlwaysFailTool));
        let mut tool_config = ToolConfig::default();
        tool_config.trace_logger = Some(build_trace_logger(&temp_dir));
        let toolset = AgentLoopToolSet {
            registry,
            config: tool_config,
        };

        let mut agent = AgentLoop::with_toolset(
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

        let response = agent
            .process_inbound_message(InboundMessage::new("cli", "user", "chat-1", "hello"), None)
            .await
            .unwrap()
            .unwrap();
        assert!(response.content.contains("maximum tool iterations"));

        let events = read_trace_events(&temp_dir);
        let failed = events
            .iter()
            .find(|event| event["event"] == "tool_call_failed")
            .unwrap();
        assert_eq!(failed["metadata"]["status"], "error");
        assert_eq!(failed["metadata"]["tool"], "fail_tool");
    }

    #[tokio::test]
    async fn test_structured_runtime_logs_capture_provider_failure() {
        let bus = MessageBus::new();
        let provider = Arc::new(FailingStreamProvider);
        let temp_dir = tempfile::tempdir().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        let file_manager = Arc::new(
            FileManager::new(FileConfig::with_path(&temp_dir.path().join("files")))
                .await
                .unwrap(),
        );

        let mut tool_config = ToolConfig::default();
        tool_config.trace_logger = Some(build_trace_logger(&temp_dir));

        let mut agent = AgentLoop::with_tools(
            bus,
            provider,
            workspace,
            None,
            Some(1),
            tool_config,
            None,
            file_manager,
        )
        .await
        .unwrap();

        let result = agent
            .process_inbound_message(InboundMessage::new("cli", "user", "chat-1", "hello"), None)
            .await;
        assert!(result.is_err());

        let events = read_trace_events(&temp_dir);
        let failed = events
            .iter()
            .find(|event| event["event"] == "llm_response_failed")
            .unwrap();
        assert_eq!(failed["metadata"]["status"], "error");
        assert_eq!(failed["metadata"]["model"], "test-model");
    }
}
