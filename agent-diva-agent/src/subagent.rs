//! Subagent management for background tasks

use std::collections::{HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tokio::task::JoinSet;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use agent_diva_core::bus::{InboundMessage, MessageBus};
use agent_diva_core::config::schema::{
    BatchSpawnRequest, MaskConfig, SubAgentResult, SubAgentStatus, TokenUsage, ToolLimits,
};
use agent_diva_core::memory::{MemoryProvider, SystemPromptRequest};
use agent_diva_core::session::TokenUsage as SessionTokenUsage;
use agent_diva_core::token_ledger::budget::check_budget_at_path;
use agent_diva_core::token_ledger::BudgetExceeded;
use agent_diva_core::token_ledger::{JsonlTokenLedger, TokenLedgerEntry};
use agent_diva_providers::base::{LLMProvider, Message};
use agent_diva_tooling::ToolRegistry;

use crate::tool_assembly::ToolAssembly;
use crate::tool_config::builtin::BuiltInToolsConfig;
use crate::tool_config::network::NetworkToolConfig;
use agent_diva_core::config::MCPServerConfig;

const MAX_CONCURRENT_SUBAGENTS: usize = 4;

/// Subagent manager for background task execution.
///
/// Subagents are lightweight agent instances that run in the background
/// to handle specific tasks. They share the same LLM provider but have
/// isolated context and a focused system prompt.
pub struct SubagentManager {
    provider: Arc<dyn LLMProvider>,
    workspace: PathBuf,
    bus: MessageBus,
    model: String,
    builtin_tools: BuiltInToolsConfig,
    network_config: Arc<RwLock<NetworkToolConfig>>,
    exec_timeout: u64,
    restrict_to_workspace: bool,
    mcp_servers: Arc<RwLock<HashMap<String, MCPServerConfig>>>,
    #[allow(dead_code)]
    parent_tool_limits: ToolLimits,
    memory_provider: Arc<dyn MemoryProvider>,
    running_tasks: Arc<tokio::sync::Mutex<HashMap<String, JoinHandle<()>>>>,
    per_task_token_budget: Option<u64>,
    token_ledger_data_root: Option<PathBuf>,
    session_token_budget_limit: Option<u64>,
    /// Mask currently active on the parent agent; subagents inherit model and defaults.
    current_mask: Arc<RwLock<Option<MaskConfig>>>,
}

/// Parent-turn context for supervised subagent runs.
#[derive(Debug, Clone, Default)]
pub struct SupervisedSubagentContext {
    pub session_key: Option<String>,
    pub trace_id: Option<String>,
    pub parent_run_id: Option<String>,
    pub token_budget_limit: Option<u64>,
}

impl SubagentManager {
    /// Create a new subagent manager
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        bus: MessageBus,
        model: Option<String>,
        builtin_tools: BuiltInToolsConfig,
        network_config: NetworkToolConfig,
        exec_timeout: Option<u64>,
        restrict_to_workspace: bool,
        mcp_servers: HashMap<String, MCPServerConfig>,
        parent_tool_limits: ToolLimits,
        memory_provider: Arc<dyn MemoryProvider>,
        per_task_token_budget: Option<u64>,
    ) -> Self {
        let model = model.unwrap_or_else(|| provider.get_default_model());
        let exec_timeout = exec_timeout.unwrap_or(30);

        Self {
            provider,
            workspace,
            bus,
            model,
            builtin_tools,
            network_config: Arc::new(RwLock::new(network_config)),
            exec_timeout,
            restrict_to_workspace,
            mcp_servers: Arc::new(RwLock::new(mcp_servers)),
            parent_tool_limits,
            memory_provider,
            running_tasks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            per_task_token_budget,
            token_ledger_data_root: None,
            session_token_budget_limit: None,
            current_mask: Arc::new(RwLock::new(None)),
        }
    }

    /// Set the mask currently active on the parent agent so subagents can inherit
    /// its model and subagent defaults.
    pub async fn set_current_mask(&self, mask: Option<MaskConfig>) {
        let mut guard = self.current_mask.write().await;
        *guard = mask;
    }

    /// Configure parent-session token ledger inheritance for supervised runs.
    pub fn with_token_ledger(
        mut self,
        data_root: PathBuf,
        session_budget_limit: Option<u64>,
    ) -> Self {
        self.token_ledger_data_root = Some(data_root);
        self.session_token_budget_limit = session_budget_limit;
        self
    }

    pub async fn update_network_config(&self, network_config: NetworkToolConfig) {
        let mut guard = self.network_config.write().await;
        *guard = network_config;
    }

    pub async fn update_mcp_servers(&self, mcp_servers: HashMap<String, MCPServerConfig>) {
        let mut guard = self.mcp_servers.write().await;
        *guard = mcp_servers;
    }

    /// Resolve the model to use for a subagent, following the priority chain:
    ///   1. Explicit spawn request model
    ///   2. Mask subagent_defaults.model
    ///   3. Mask model
    ///   4. Global default (self.model)
    pub fn resolve_model(
        spawn_model: Option<&str>,
        mask: Option<&MaskConfig>,
        global_default: &str,
    ) -> String {
        // 1. Explicit spawn request
        if let Some(m) = spawn_model {
            if !m.is_empty() {
                return m.to_string();
            }
        }

        // 2. Mask subagent_defaults.model
        if let Some(mask_cfg) = mask {
            if let Some(ref m) = mask_cfg.subagent_defaults.model {
                if !m.is_empty() {
                    return m.clone();
                }
            }

            // 3. Mask model
            if let Some(ref m) = mask_cfg.model {
                if !m.is_empty() {
                    return m.clone();
                }
            }
        }

        // 4. Global default
        global_default.to_string()
    }

    /// Resolve max iterations for a subagent, following the priority chain:
    ///   1. Explicit spawn request limit
    ///   2. Mask subagent_defaults.max_iterations
    ///   3. Default (30)
    pub fn resolve_max_iterations(spawn_limit: Option<u32>, mask: Option<&MaskConfig>) -> u32 {
        const DEFAULT_MAX_ITERATIONS: u32 = 30;

        // 1. Explicit spawn request
        if let Some(limit) = spawn_limit {
            if limit > 0 {
                return limit;
            }
        }

        // 2. Mask subagent_defaults.max_iterations
        if let Some(mask_cfg) = mask {
            if let Some(limit) = mask_cfg.subagent_defaults.max_iterations {
                if limit > 0 {
                    return limit;
                }
            }
        }

        // 3. Default
        DEFAULT_MAX_ITERATIONS
    }

    /// Spawn a subagent to execute a task in the background.
    ///
    /// # Arguments
    /// * `task` - The task description for the subagent
    /// * `label` - Optional human-readable label for the task
    /// * `origin_channel` - The channel to announce results to
    /// * `origin_chat_id` - The chat ID to announce results to
    ///
    /// # Returns
    /// Status message indicating the subagent was started
    pub async fn spawn(
        &self,
        task: String,
        label: Option<String>,
        origin_channel: String,
        origin_chat_id: String,
    ) -> Result<String> {
        let task_id = Uuid::new_v4().to_string()[..8].to_string();
        let display_label = Self::display_label(&task, label);

        let provider = Arc::clone(&self.provider);
        let workspace = self.workspace.clone();
        let bus = self.bus.clone();
        let builtin_tools = self.builtin_tools.clone();
        let network_config = self.network_config.read().await.clone();
        let exec_timeout = self.exec_timeout;
        let restrict_to_workspace = self.restrict_to_workspace;
        let mcp_servers = self.mcp_servers.read().await.clone();
        let memory_provider = Arc::clone(&self.memory_provider);

        let mask = self.current_mask.read().await.clone();
        let model = Self::resolve_model(None, mask.as_ref(), &self.model);
        let max_iterations = Self::resolve_max_iterations(None, mask.as_ref());

        let task_id_clone = task_id.clone();
        let display_label_clone = display_label.clone();
        let running_tasks = Arc::clone(&self.running_tasks);
        let per_task_token_budget = self.per_task_token_budget;

        // Create background task
        let bg_task = tokio::spawn(async move {
            Self::run_subagent(
                task_id_clone.clone(),
                task.clone(),
                display_label_clone.clone(),
                origin_channel,
                origin_chat_id,
                provider,
                workspace,
                bus.clone(),
                model,
                builtin_tools,
                network_config,
                exec_timeout,
                restrict_to_workspace,
                mcp_servers,
                memory_provider,
                max_iterations,
                per_task_token_budget,
            )
            .await;

            // Cleanup when done
            let mut tasks = running_tasks.lock().await;
            tasks.remove(&task_id_clone);
        });

        // Store the task handle
        let mut tasks = self.running_tasks.lock().await;
        tasks.insert(task_id.clone(), bg_task);
        drop(tasks);

        info!("Spawned subagent [{}]: {}", task_id, display_label);
        Ok(format!(
            "Subagent [{}] started (id: {}). I'll notify you when it completes.",
            display_label, task_id
        ))
    }

    /// Execute a subagent as a supervised foreground run.
    ///
    /// Unlike [`Self::spawn`], this waits for the real subagent work to finish
    /// so the supervised run store reflects the true success or failure.
    pub async fn run_supervised(
        &self,
        task: String,
        label: Option<String>,
        origin_channel: String,
        origin_chat_id: String,
        context: SupervisedSubagentContext,
    ) -> Result<String> {
        let task_id = Uuid::new_v4().to_string()[..8].to_string();
        let display_label = Self::display_label(&task, label);
        let network_config = self.network_config.read().await.clone();
        let mcp_servers = self.mcp_servers.read().await.clone();

        let mask = self.current_mask.read().await.clone();
        let model = Self::resolve_model(None, mask.as_ref(), &self.model);
        let max_iterations = Self::resolve_max_iterations(None, mask.as_ref());

        info!(
            "Supervised subagent [{}] starting task: {}",
            task_id, display_label
        );

        self.enforce_parent_session_budget(&context)?;

        let result = Self::execute_subagent_task(
            &task_id,
            &task,
            &self.provider,
            &self.workspace,
            &model,
            &self.builtin_tools,
            &network_config,
            self.exec_timeout,
            self.restrict_to_workspace,
            &mcp_servers,
            Arc::clone(&self.memory_provider),
            max_iterations,
            self.per_task_token_budget,
        )
        .await;

        match result {
            Ok((content, usage)) => {
                info!("Supervised subagent [{}] completed successfully", task_id);
                if !usage.is_empty() {
                    debug!("Supervised subagent [{}] token usage: {:?}", task_id, usage);
                }
                self.append_parent_session_usage(&context, &usage)?;
                Self::announce_result(
                    &task_id,
                    &display_label,
                    &task,
                    &content,
                    &origin_channel,
                    &origin_chat_id,
                    "ok",
                    &self.bus,
                )
                .await;
                Ok(content)
            }
            Err(error) => {
                let error_msg = format!("Error: {}", error);
                error!("Supervised subagent [{}] failed: {}", task_id, error);
                Self::announce_result(
                    &task_id,
                    &display_label,
                    &task,
                    &error_msg,
                    &origin_channel,
                    &origin_chat_id,
                    "error",
                    &self.bus,
                )
                .await;
                Err(error)
            }
        }
    }

    fn enforce_parent_session_budget(&self, context: &SupervisedSubagentContext) -> Result<()> {
        let Some(session_key) = context.session_key.as_deref() else {
            return Ok(());
        };
        let limit = context
            .token_budget_limit
            .or(self.session_token_budget_limit);
        if let (Some(data_root), Some(limit)) = (&self.token_ledger_data_root, limit) {
            check_budget_at_path(data_root, session_key, limit)?;
        }
        Ok(())
    }

    fn append_parent_session_usage(
        &self,
        context: &SupervisedSubagentContext,
        usage: &HashMap<String, i64>,
    ) -> Result<()> {
        let Some(session_key) = context.session_key.as_deref() else {
            return Ok(());
        };
        let Some(data_root) = &self.token_ledger_data_root else {
            return Ok(());
        };
        let token_usage = SessionTokenUsage {
            prompt_tokens: usage.get("prompt_tokens").copied().unwrap_or(0).max(0) as u32,
            completion_tokens: usage.get("completion_tokens").copied().unwrap_or(0).max(0) as u32,
            total_tokens: usage.get("total_tokens").copied().unwrap_or(0).max(0) as u32,
        };
        if token_usage.total_tokens == 0 {
            return Ok(());
        }
        let ledger = JsonlTokenLedger::new(data_root)?;
        ledger.append(TokenLedgerEntry::from_usage(
            session_key,
            self.model.clone(),
            &token_usage,
        ))?;
        Ok(())
    }

    /// Spawn a batch of isolated subagent tasks in parallel.
    ///
    /// Each task runs in its own context with no personality/soul injection
    /// and no inter-task communication. Tasks that exceed the timeout are
    /// marked as `Timeout`. All results (successes and failures) are returned.
    ///
    /// # Arguments
    /// * `request` - Batch spawn request containing the tasks
    ///
    /// # Returns
    /// A `Vec<SubAgentResult>` — one per task, in arbitrary completion order.
    pub async fn spawn_batch(&self, request: BatchSpawnRequest) -> Vec<SubAgentResult> {
        let provider = Arc::clone(&self.provider);
        let workspace = self.workspace.clone();
        let builtin_tools = self.builtin_tools.clone();
        let network_config = self.network_config.read().await.clone();
        let exec_timeout = self.exec_timeout;
        let restrict_to_workspace = self.restrict_to_workspace;
        let mcp_servers = self.mcp_servers.read().await.clone();
        let memory_provider = Arc::clone(&self.memory_provider);
        let running_tasks = Arc::clone(&self.running_tasks);
        let per_task_token_budget = self.per_task_token_budget;

        let mask = self.current_mask.read().await.clone();
        let model = Self::resolve_model(None, mask.as_ref(), &self.model);
        let resolved_max_iterations = Self::resolve_max_iterations(None, mask.as_ref());

        // Resolve max_iterations once for the entire batch
        let max_iterations = request.max_iterations.unwrap_or(resolved_max_iterations);

        let mut join_set = JoinSet::new();
        let mut tasks = VecDeque::from(request.tasks);
        let max_concurrent = MAX_CONCURRENT_SUBAGENTS.max(1);

        // Register a sentinel in running_tasks so cancellation has visibility
        let batch_id = format!("batch-{}", &Uuid::new_v4().to_string()[..8]);
        {
            let mut tasks_map = running_tasks.lock().await;
            tasks_map.insert(batch_id.clone(), tokio::spawn(async {}));
        }

        for _ in 0..max_concurrent {
            let Some(task) = tasks.pop_front() else {
                break;
            };
            let provider = Arc::clone(&provider);
            let workspace = workspace.clone();
            let model = model.clone();
            let builtin_tools = builtin_tools.clone();
            let network_config = network_config.clone();
            let mcp_servers = mcp_servers.clone();
            let memory_provider = Arc::clone(&memory_provider);

            join_set.spawn(async move {
                Self::run_isolated_subagent(
                    task.id,
                    task.goal,
                    task.context,
                    provider,
                    workspace,
                    &model,
                    &builtin_tools,
                    &network_config,
                    exec_timeout,
                    restrict_to_workspace,
                    &mcp_servers,
                    memory_provider.clone(),
                    max_iterations,
                    per_task_token_budget,
                )
                .await
            });
        }

        let mut results = Vec::new();
        while let Some(join_result) = join_set.join_next().await {
            match join_result {
                Ok(subagent_result) => results.push(subagent_result),
                Err(join_err) => {
                    // Task panicked — report as Error
                    warn!("Batch subagent task panicked: {}", join_err);
                    results.push(SubAgentResult {
                        task_id: "unknown".to_string(),
                        status: SubAgentStatus::Error,
                        summary: Some(format!("Task panicked: {}", join_err)),
                        elapsed_ms: 0,
                        tool_call_count: 0,
                        token_usage: None,
                        tool_trace: None,
                    });
                }
            }

            if let Some(task) = tasks.pop_front() {
                let provider = Arc::clone(&provider);
                let workspace = workspace.clone();
                let model = model.clone();
                let builtin_tools = builtin_tools.clone();
                let network_config = network_config.clone();
                let mcp_servers = mcp_servers.clone();
                let memory_provider = Arc::clone(&memory_provider);

                join_set.spawn(async move {
                    Self::run_isolated_subagent(
                        task.id,
                        task.goal,
                        task.context,
                        provider,
                        workspace,
                        &model,
                        &builtin_tools,
                        &network_config,
                        exec_timeout,
                        restrict_to_workspace,
                        &mcp_servers,
                        memory_provider.clone(),
                        max_iterations,
                        per_task_token_budget,
                    )
                    .await
                });
            }
        }

        // Remove the batch sentinel from running_tasks
        {
            let mut tasks_map = running_tasks.lock().await;
            tasks_map.remove(&batch_id);
        }

        results
    }

    /// Run a single isolated subagent task with timeout enforcement.
    ///
    /// Uses a minimal task-only prompt (no personality/soul). If the task
    /// exceeds `exec_timeout` seconds, returns `SubAgentStatus::Timeout`.
    #[allow(clippy::too_many_arguments)]
    async fn run_isolated_subagent(
        task_id: String,
        goal: String,
        context: Option<String>,
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        model: &str,
        builtin_tools: &BuiltInToolsConfig,
        network_config: &NetworkToolConfig,
        exec_timeout: u64,
        restrict_to_workspace: bool,
        mcp_servers: &HashMap<String, MCPServerConfig>,
        memory_provider: Arc<dyn MemoryProvider>,
        max_iterations: u32,
        per_task_token_budget: Option<u64>,
    ) -> SubAgentResult {
        let start = Instant::now();
        let task_prompt = match &context {
            Some(ctx) => format!("{}\n\nAdditional context:\n{}", goal, ctx),
            None => goal.clone(),
        };

        let timeout_duration = std::time::Duration::from_secs(exec_timeout);

        let exec_result = tokio::time::timeout(
            timeout_duration,
            Self::execute_isolated_task(
                &task_id,
                &task_prompt,
                &provider,
                &workspace,
                model,
                builtin_tools,
                network_config,
                exec_timeout,
                restrict_to_workspace,
                mcp_servers,
                memory_provider,
                max_iterations,
                per_task_token_budget,
            ),
        )
        .await;

        let elapsed_ms = start.elapsed().as_millis() as u64;

        match exec_result {
            Ok(Ok((summary, tool_call_count, tool_trace, usage))) => {
                info!(
                    "Batch subagent [{}] completed in {}ms ({} tool calls)",
                    task_id, elapsed_ms, tool_call_count
                );
                SubAgentResult {
                    task_id,
                    status: SubAgentStatus::Ok,
                    summary: Some(summary),
                    elapsed_ms,
                    tool_call_count,
                    token_usage: extract_token_usage(&usage),
                    tool_trace: Some(tool_trace),
                }
            }
            Ok(Err(e)) => {
                error!("Batch subagent [{}] failed: {}", task_id, e);
                SubAgentResult {
                    task_id,
                    status: SubAgentStatus::Error,
                    summary: Some(format!("Error: {}", e)),
                    elapsed_ms,
                    tool_call_count: 0,
                    token_usage: None,
                    tool_trace: None,
                }
            }
            Err(_elapsed) => {
                warn!(
                    "Batch subagent [{}] timed out after {}ms",
                    task_id, elapsed_ms
                );
                SubAgentResult {
                    task_id,
                    status: SubAgentStatus::Timeout,
                    summary: Some(format!("Task timed out after {}s", exec_timeout)),
                    elapsed_ms,
                    tool_call_count: 0,
                    token_usage: None,
                    tool_trace: None,
                }
            }
        }
    }

    /// Execute an isolated task with the LLM and tools.
    ///
    /// Returns `(summary, tool_call_count, tool_trace, token_usage)` on success.
    #[allow(clippy::too_many_arguments)]
    async fn execute_isolated_task(
        task_id: &str,
        task: &str,
        provider: &Arc<dyn LLMProvider>,
        workspace: &Path,
        model: &str,
        builtin_tools: &BuiltInToolsConfig,
        network_config: &NetworkToolConfig,
        exec_timeout: u64,
        restrict_to_workspace: bool,
        mcp_servers: &HashMap<String, MCPServerConfig>,
        _memory_provider: Arc<dyn MemoryProvider>,
        max_iterations: u32,
        per_task_token_budget: Option<u64>,
    ) -> Result<(String, u32, Vec<String>, HashMap<String, i64>)> {
        let tools: ToolRegistry = ToolAssembly::new(workspace.to_path_buf())
            .builtin(builtin_tools.clone())
            .with_network_config(network_config.clone())
            .with_exec_timeout(exec_timeout)
            .restrict_to_workspace(restrict_to_workspace)
            .mcp_servers(mcp_servers.clone())
            .build_subagent_registry();

        let system_prompt = Self::build_isolated_subagent_prompt(task, workspace);
        let mut messages = vec![
            Message::system(system_prompt),
            Message::user(task.to_string()),
        ];

        let mut iteration = 0;
        let mut final_result: Option<String> = None;
        let mut final_usage: HashMap<String, i64> = HashMap::new();
        let mut tool_call_count: u32 = 0;
        let mut tool_trace: Vec<String> = Vec::new();
        let mut cumulative_tokens: u64 = 0;

        while iteration < max_iterations {
            iteration += 1;

            // Check per-task token budget before making the LLM call
            if let Some(budget) = per_task_token_budget {
                if cumulative_tokens >= budget {
                    return Err(BudgetExceeded {
                        session_id: task_id.to_string(),
                        used: cumulative_tokens,
                        limit: budget,
                    }
                    .into());
                }
            }

            let response = provider
                .chat(
                    messages.clone(),
                    Some(tools.get_definitions()),
                    agent_diva_providers::ToolChoiceMode::Auto,
                    Some(model.to_string()),
                    2000,
                    0.7,
                )
                .await?;

            // Capture cumulative usage for the whole task.
            if !response.usage.is_empty() {
                accumulate_usage(&mut final_usage, &response.usage);
            }

            // Track cumulative tokens for budget enforcement
            let total: i64 = response.usage.get("total_tokens").copied().unwrap_or(0);
            cumulative_tokens = cumulative_tokens.saturating_add(total.max(0) as u64);

            if response.has_tool_calls() {
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: agent_diva_providers::MessageContent::Text(
                        response.content.clone().unwrap_or_default(),
                    ),
                    name: None,
                    tool_call_id: None,
                    tool_calls: Some(response.tool_calls.clone()),
                    reasoning_content: response.reasoning_content.clone(),
                    thinking_blocks: None,
                });

                for tool_call in &response.tool_calls {
                    let args_json = serde_json::to_value(&tool_call.arguments)?;
                    let args_str = serde_json::to_string(&tool_call.arguments)?;
                    debug!(
                        "Batch subagent [{}] executing: {} with arguments: {}",
                        task_id, tool_call.name, args_str
                    );
                    tool_call_count += 1;
                    tool_trace.push(tool_call.name.clone());
                    match tools.execute(&tool_call.name, args_json).await {
                        Ok(text) => messages.push(Message::tool(text, tool_call.id.clone())),
                        Err(e) => messages
                            .push(Message::tool(format!("Error: {}", e), tool_call.id.clone())),
                    }
                }
            } else {
                final_result = response.content;
                break;
            }
        }

        let summary = final_result
            .unwrap_or_else(|| "Task completed but no final response was generated.".to_string());
        Ok((summary, tool_call_count, tool_trace, final_usage))
    }

    /// Execute the subagent task with LLM and tools
    ///
    /// Returns `(final_text, token_usage)` on success.
    #[allow(clippy::too_many_arguments)]
    async fn execute_subagent_task(
        task_id: &str,
        task: &str,
        provider: &Arc<dyn LLMProvider>,
        workspace: &Path,
        model: &str,
        builtin_tools: &BuiltInToolsConfig,
        network_config: &NetworkToolConfig,
        exec_timeout: u64,
        restrict_to_workspace: bool,
        mcp_servers: &HashMap<String, MCPServerConfig>,
        memory_provider: Arc<dyn MemoryProvider>,
        max_iterations: u32,
        per_task_token_budget: Option<u64>,
    ) -> Result<(String, HashMap<String, i64>)> {
        let tools: ToolRegistry = ToolAssembly::new(workspace.to_path_buf())
            .builtin(builtin_tools.clone())
            .with_network_config(network_config.clone())
            .with_exec_timeout(exec_timeout)
            .restrict_to_workspace(restrict_to_workspace)
            .mcp_servers(mcp_servers.clone())
            .build_subagent_registry();

        // Build messages with subagent-specific prompt
        let system_prompt = Self::build_subagent_prompt(task, workspace, memory_provider.as_ref());
        let mut messages = vec![
            Message::system(system_prompt),
            Message::user(task.to_string()),
        ];

        // Run agent loop (limited iterations)
        let mut iteration = 0;
        let mut final_result: Option<String> = None;
        let mut final_usage: HashMap<String, i64> = HashMap::new();
        let mut cumulative_tokens: u64 = 0;

        while iteration < max_iterations {
            iteration += 1;

            // Check per-task token budget before making the LLM call
            if let Some(budget) = per_task_token_budget {
                if cumulative_tokens >= budget {
                    return Err(BudgetExceeded {
                        session_id: task_id.to_string(),
                        used: cumulative_tokens,
                        limit: budget,
                    }
                    .into());
                }
            }

            let response = provider
                .chat(
                    messages.clone(),
                    Some(tools.get_definitions()),
                    agent_diva_providers::ToolChoiceMode::Auto,
                    Some(model.to_string()),
                    2000,
                    0.7,
                )
                .await?;

            // Capture cumulative usage for the whole task.
            if !response.usage.is_empty() {
                accumulate_usage(&mut final_usage, &response.usage);
            }

            // Track cumulative tokens for budget enforcement
            let total: i64 = response.usage.get("total_tokens").copied().unwrap_or(0);
            cumulative_tokens = cumulative_tokens.saturating_add(total.max(0) as u64);

            if response.has_tool_calls() {
                // Add assistant message with tool calls
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: agent_diva_providers::MessageContent::Text(
                        response.content.clone().unwrap_or_default(),
                    ),
                    name: None,
                    tool_call_id: None,
                    tool_calls: Some(response.tool_calls.clone()),
                    reasoning_content: response.reasoning_content.clone(),
                    thinking_blocks: None,
                });

                // Execute tools
                for tool_call in &response.tool_calls {
                    let args_json = serde_json::to_value(&tool_call.arguments)?;
                    let args_str = serde_json::to_string(&tool_call.arguments)?;
                    debug!(
                        "Subagent [{}] executing: {} with arguments: {}",
                        task_id, tool_call.name, args_str
                    );
                    match tools.execute(&tool_call.name, args_json).await {
                        Ok(text) => messages.push(Message::tool(text, tool_call.id.clone())),
                        Err(e) => messages
                            .push(Message::tool(format!("Error: {}", e), tool_call.id.clone())),
                    }
                }
            } else {
                final_result = response.content;
                break;
            }
        }

        let result_text = final_result
            .unwrap_or_else(|| "Task completed but no final response was generated.".to_string());
        Ok((result_text, final_usage))
    }

    /// Announce the subagent result to the main agent via the message bus
    #[allow(clippy::too_many_arguments)]
    async fn announce_result(
        task_id: &str,
        label: &str,
        task: &str,
        result: &str,
        origin_channel: &str,
        origin_chat_id: &str,
        status: &str,
        bus: &MessageBus,
    ) {
        let status_text = if status == "ok" {
            "completed successfully"
        } else {
            "failed"
        };

        let announce_content = format!(
            "[Subagent '{}' {}]\n\nTask: {}\n\nResult:\n{}\n\nSummarize this naturally for the user. Keep it brief (1-2 sentences). Do not mention technical details like \"subagent\" or task IDs.",
            label, status_text, task, result
        );

        // Inject as system message to trigger main agent
        // Use the origin channel/chat_id directly so the response routes back correctly
        let msg = InboundMessage::new(origin_channel, "subagent", origin_chat_id, announce_content);

        if let Err(e) = bus.publish_inbound(msg) {
            error!("Failed to announce subagent result: {}", e);
        }

        debug!(
            "Subagent [{}] announced result to {}:{}",
            task_id, origin_channel, origin_chat_id
        );
    }

    /// Build a focused system prompt for the subagent
    fn build_subagent_prompt(
        task: &str,
        workspace: &Path,
        memory_provider: &dyn MemoryProvider,
    ) -> String {
        let authority_context = Self::build_applied_authority_context(workspace, memory_provider);
        format!(
            r#"# Subagent

You are a subagent spawned by the main agent to complete a specific task.

## Your Task
{}

## Applied Authority Context
{}

## Rules
1. Stay focused - complete only the assigned task, nothing else
2. Your final response will be reported back to the main agent
3. Do not initiate conversations or take on side tasks
4. Be concise but informative in your findings

## What You Can Do
- Read and write files in the workspace
- Execute shell commands
- Search the web and fetch web pages
- Complete the task thoroughly

## What You Cannot Do
- Send messages directly to users (no message tool available)
- Ask the user questions (no ask_user tool available)
- Spawn other subagents
- Access the main agent's conversation history

## Workspace
Your workspace is at: {}

When you have completed the task, provide a clear summary of your findings or actions."#,
            task,
            authority_context,
            workspace.display()
        )
    }

    /// Build an isolated subagent prompt with no personality/soul injection.
    ///
    /// Used by `spawn_batch` — children get only the task context, no inherited
    /// identity, and no inter-task communication channel.
    fn build_isolated_subagent_prompt(task: &str, workspace: &Path) -> String {
        format!(
            r#"# Isolated Subagent

You are an isolated subagent. You have no personality or identity of your own.
Your only purpose is to complete the assigned task.

## Your Task
{}

## Rules
1. Complete only the assigned task, nothing else
2. Be concise but thorough in your findings
3. Do not attempt to communicate with other subagents or the user

## What You Can Do
- Read and write files in the workspace
- Execute shell commands
- Search the web and fetch web pages

## Workspace
Your workspace is at: {}

When you have completed the task, provide a clear summary of your findings or actions."#,
            task,
            workspace.display()
        )
    }

    fn build_applied_authority_context(
        workspace: &Path,
        memory_provider: &dyn MemoryProvider,
    ) -> String {
        match memory_provider.system_prompt_block(&SystemPromptRequest {
            workspace_root: workspace.to_path_buf(),
        }) {
            Ok(response) => response.prompt_block.map_or_else(
                || "No applied Laputa authority context is available.".to_string(),
                |block| block.markdown,
            ),
            Err(_) => "Applied Laputa authority context could not be read. Continue with the assigned task and do not treat legacy authority files as inherited identity.".to_string(),
        }
    }

    fn display_label(task: &str, label: Option<String>) -> String {
        label.unwrap_or_else(|| {
            if task.len() > 30 {
                let mut end = 30;
                while !task.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &task[..end])
            } else {
                task.to_string()
            }
        })
    }

    /// Execute the subagent task and announce the result
    #[allow(clippy::too_many_arguments)]
    async fn run_subagent(
        task_id: String,
        task: String,
        label: String,
        origin_channel: String,
        origin_chat_id: String,
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        bus: MessageBus,
        model: String,
        builtin_tools: BuiltInToolsConfig,
        network_config: NetworkToolConfig,
        exec_timeout: u64,
        restrict_to_workspace: bool,
        mcp_servers: HashMap<String, MCPServerConfig>,
        memory_provider: Arc<dyn MemoryProvider>,
        max_iterations: u32,
        per_task_token_budget: Option<u64>,
    ) {
        info!("Subagent [{}] starting task: {}", task_id, label);

        let result = Self::execute_subagent_task(
            &task_id,
            &task,
            &provider,
            &workspace,
            &model,
            &builtin_tools,
            &network_config,
            exec_timeout,
            restrict_to_workspace,
            &mcp_servers,
            memory_provider,
            max_iterations,
            per_task_token_budget,
        )
        .await;

        let (final_result, status) = match result {
            Ok((content, usage)) => {
                info!("Subagent [{}] completed successfully", task_id);
                if !usage.is_empty() {
                    debug!("Subagent [{}] token usage: {:?}", task_id, usage);
                }
                (content, "ok")
            }
            Err(e) => {
                let error_msg = format!("Error: {}", e);
                error!("Subagent [{}] failed: {}", task_id, e);
                (error_msg, "error")
            }
        };

        Self::announce_result(
            &task_id,
            &label,
            &task,
            &final_result,
            &origin_channel,
            &origin_chat_id,
            status,
            &bus,
        )
        .await;
    }

    /// Get the number of currently running subagents
    pub async fn get_running_count(&self) -> usize {
        let tasks = self.running_tasks.lock().await;
        tasks.len()
    }

    #[cfg(test)]
    pub(crate) fn per_task_token_budget_for_test(&self) -> Option<u64> {
        self.per_task_token_budget
    }
}

/// Convert an LLM provider usage map (`HashMap<String, i64>`) into a
/// structured [`TokenUsage`], returning `None` when the map is empty.
fn extract_token_usage(usage: &HashMap<String, i64>) -> Option<TokenUsage> {
    if usage.is_empty() {
        return None;
    }
    Some(TokenUsage {
        prompt_tokens: usage.get("prompt_tokens").copied().unwrap_or(0).max(0) as u32,
        completion_tokens: usage.get("completion_tokens").copied().unwrap_or(0).max(0) as u32,
        total_tokens: usage.get("total_tokens").copied().unwrap_or(0).max(0) as u32,
    })
}

fn accumulate_usage(total_usage: &mut HashMap<String, i64>, delta: &HashMap<String, i64>) {
    for (key, value) in delta {
        let entry = total_usage.entry(key.clone()).or_insert(0);
        *entry = entry.saturating_add((*value).max(0));
    }
}

#[cfg(test)]
mod tests {
    use super::{accumulate_usage, extract_token_usage, SubagentManager};
    use agent_diva_core::config::schema::{
        BatchSpawnRequest, MaskConfig, SubAgentStatus, SubAgentTask, SubagentDefaults, TokenUsage,
    };
    use agent_diva_core::memory::{
        MemoryProvider, StartupInjectionShape, SystemPromptBlock, SystemPromptRequest,
        SystemPromptResponse,
    };

    struct TestMemoryProvider;

    #[async_trait::async_trait]
    impl MemoryProvider for TestMemoryProvider {
        fn system_prompt_block(
            &self,
            _request: &SystemPromptRequest,
        ) -> agent_diva_core::Result<SystemPromptResponse> {
            Ok(SystemPromptResponse::ready(SystemPromptBlock {
                shape: StartupInjectionShape::CompactRenderedMarkdown,
                markdown: "## Applied Laputa Authority\n- provenance: test".to_string(),
            }))
        }

        async fn prefetch(
            &self,
            _request: agent_diva_core::memory::PrefetchRequest,
        ) -> agent_diva_core::Result<agent_diva_core::memory::PrefetchResponse> {
            Ok(agent_diva_core::memory::PrefetchResponse::default())
        }

        async fn sync_turn(
            &self,
            _request: agent_diva_core::memory::SyncTurnRequest,
        ) -> agent_diva_core::Result<agent_diva_core::memory::SyncTurnResponse> {
            Ok(agent_diva_core::memory::SyncTurnResponse::default())
        }

        async fn on_session_end(
            &self,
            _request: agent_diva_core::memory::SessionEndRequest,
        ) -> agent_diva_core::Result<agent_diva_core::memory::SessionEndResponse> {
            Ok(agent_diva_core::memory::SessionEndResponse::default())
        }
    }

    // ── resolve_model tests ────────────────────────────────────────────────

    #[test]
    fn explicit_model_takes_priority() {
        let mask = MaskConfig {
            name: "test".to_string(),
            model: Some("mask-model".to_string()),
            subagent_defaults: SubagentDefaults {
                model: Some("subagent-model".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let result =
            SubagentManager::resolve_model(Some("explicit-model"), Some(&mask), "global-default");
        assert_eq!(result, "explicit-model");
    }

    #[test]
    fn falls_back_to_mask_subagent_defaults_model() {
        let mask = MaskConfig {
            name: "test".to_string(),
            model: Some("mask-model".to_string()),
            subagent_defaults: SubagentDefaults {
                model: Some("subagent-model".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = SubagentManager::resolve_model(None, Some(&mask), "global-default");
        assert_eq!(result, "subagent-model");
    }

    #[test]
    fn falls_back_to_mask_model() {
        let mask = MaskConfig {
            name: "test".to_string(),
            model: Some("mask-model".to_string()),
            subagent_defaults: SubagentDefaults::default(),
            ..Default::default()
        };
        let result = SubagentManager::resolve_model(None, Some(&mask), "global-default");
        assert_eq!(result, "mask-model");
    }

    #[test]
    fn falls_back_to_global_default() {
        let result = SubagentManager::resolve_model(None, None, "global-default");
        assert_eq!(result, "global-default");
    }

    #[test]
    fn empty_explicit_model_skips_to_next() {
        let mask = MaskConfig {
            name: "test".to_string(),
            model: Some("mask-model".to_string()),
            subagent_defaults: SubagentDefaults::default(),
            ..Default::default()
        };
        let result = SubagentManager::resolve_model(Some(""), Some(&mask), "global-default");
        assert_eq!(result, "mask-model");
    }

    // ── resolve_max_iterations tests ───────────────────────────────────────

    #[test]
    fn explicit_limit_takes_priority() {
        let mask = MaskConfig {
            name: "test".to_string(),
            subagent_defaults: SubagentDefaults {
                max_iterations: Some(50),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = SubagentManager::resolve_max_iterations(Some(10), Some(&mask));
        assert_eq!(result, 10);
    }

    #[test]
    fn falls_back_to_mask_subagent_defaults_max_iterations() {
        let mask = MaskConfig {
            name: "test".to_string(),
            subagent_defaults: SubagentDefaults {
                max_iterations: Some(50),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = SubagentManager::resolve_max_iterations(None, Some(&mask));
        assert_eq!(result, 50);
    }

    #[test]
    fn falls_back_to_default_max_iterations() {
        let result = SubagentManager::resolve_max_iterations(None, None);
        assert_eq!(result, 30);
    }

    #[test]
    fn zero_explicit_limit_skips_to_next() {
        let mask = MaskConfig {
            name: "test".to_string(),
            subagent_defaults: SubagentDefaults {
                max_iterations: Some(50),
                ..Default::default()
            },
            ..Default::default()
        };
        let result = SubagentManager::resolve_max_iterations(Some(0), Some(&mask));
        assert_eq!(result, 50);
    }

    #[test]
    fn test_build_subagent_prompt_includes_applied_laputa_authority() {
        let temp = tempfile::tempdir().unwrap();
        let prompt = SubagentManager::build_subagent_prompt(
            "analyze logs",
            temp.path(),
            &TestMemoryProvider,
        );
        assert!(prompt.contains("## Applied Authority Context"));
        assert!(prompt.contains("## Applied Laputa Authority"));
        assert!(prompt.contains("provenance: test"));
    }

    #[test]
    fn test_build_subagent_prompt_excludes_legacy_identity_files() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("SOUL.md"), "# Soul\n\nKeep concise.").unwrap();
        std::fs::write(temp.path().join("IDENTITY.md"), "# Identity\n\nAgent Diva.").unwrap();
        std::fs::write(
            temp.path().join("USER.md"),
            "# User\n\nPrefer direct replies.",
        )
        .unwrap();

        let prompt = SubagentManager::build_subagent_prompt(
            "analyze logs",
            temp.path(),
            &TestMemoryProvider,
        );
        assert!(prompt.contains("## Applied Laputa Authority"));
        assert!(!prompt.contains("### SOUL.md"));
        assert!(!prompt.contains("### IDENTITY.md"));
        assert!(!prompt.contains("### USER.md"));
        assert!(!prompt.contains("Keep concise."));
        assert!(!prompt.contains("Agent Diva."));
    }

    // ── build_isolated_subagent_prompt tests ──────────────────────────────

    #[test]
    fn test_isolated_prompt_has_no_soul() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("SOUL.md"), "# Soul\n\nYou are Diva.").unwrap();

        let prompt = SubagentManager::build_isolated_subagent_prompt("summarize file", temp.path());
        assert!(!prompt.contains("SOUL.md"));
        assert!(!prompt.contains("You are Diva"));
        assert!(!prompt.contains("Applied Authority Context"));
        assert!(prompt.contains("no personality"));
        assert!(prompt.contains("summarize file"));
    }

    #[test]
    fn test_isolated_prompt_includes_workspace_path() {
        let temp = tempfile::tempdir().unwrap();
        let prompt = SubagentManager::build_isolated_subagent_prompt("task", temp.path());
        assert!(prompt.contains(&temp.path().display().to_string()));
    }

    // ── spawn_batch unit-level contract tests ─────────────────────────────

    #[test]
    fn batch_spawn_request_tasks_are_preserved() {
        let request = BatchSpawnRequest {
            tasks: vec![
                SubAgentTask {
                    id: "t1".to_string(),
                    goal: "Goal 1".to_string(),
                    context: None,
                },
                SubAgentTask {
                    id: "t2".to_string(),
                    goal: "Goal 2".to_string(),
                    context: Some("extra info".to_string()),
                },
            ],
            max_iterations: None,
        };
        assert_eq!(request.tasks.len(), 2);
        assert_eq!(request.tasks[0].id, "t1");
        assert_eq!(request.tasks[1].context.as_deref(), Some("extra info"));
    }

    #[test]
    fn batch_spawn_empty_tasks() {
        let request = BatchSpawnRequest {
            tasks: vec![],
            max_iterations: None,
        };
        assert!(request.tasks.is_empty());
    }

    #[test]
    fn subagent_result_has_required_fields() {
        // Verify the SubAgentResult structure we produce has all required fields
        let result = agent_diva_core::config::schema::SubAgentResult {
            task_id: "batch-task-1".to_string(),
            status: SubAgentStatus::Ok,
            summary: Some("Done".to_string()),
            elapsed_ms: 100,
            tool_call_count: 3,
            token_usage: Some(TokenUsage {
                prompt_tokens: 50,
                completion_tokens: 100,
                total_tokens: 150,
            }),
            tool_trace: Some(vec!["read_file".to_string(), "write_file".to_string()]),
        };
        assert_eq!(result.task_id, "batch-task-1");
        assert_eq!(result.status, SubAgentStatus::Ok);
        assert_eq!(result.tool_call_count, 3);
        assert_eq!(
            result.token_usage,
            Some(TokenUsage {
                prompt_tokens: 50,
                completion_tokens: 100,
                total_tokens: 150,
            })
        );
        assert!(result.tool_trace.is_some());
        assert_eq!(result.tool_trace.unwrap().len(), 2);
    }

    #[test]
    fn timeout_status_is_distinct() {
        let result = agent_diva_core::config::schema::SubAgentResult {
            task_id: "slow-task".to_string(),
            status: SubAgentStatus::Timeout,
            summary: Some("Task timed out after 30s".to_string()),
            elapsed_ms: 30000,
            tool_call_count: 0,
            token_usage: None,
            tool_trace: None,
        };
        assert_eq!(result.status, SubAgentStatus::Timeout);
    }

    // ── extract_token_usage tests ─────────────────────────────────────────

    #[test]
    fn test_extract_token_usage_empty_returns_none() {
        let empty: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
        assert_eq!(extract_token_usage(&empty), None);
    }

    #[test]
    fn test_extract_token_usage_full_map() {
        let mut usage = std::collections::HashMap::new();
        usage.insert("prompt_tokens".to_string(), 50);
        usage.insert("completion_tokens".to_string(), 100);
        usage.insert("total_tokens".to_string(), 150);
        let result = extract_token_usage(&usage);
        assert_eq!(
            result,
            Some(TokenUsage {
                prompt_tokens: 50,
                completion_tokens: 100,
                total_tokens: 150,
            })
        );
    }

    #[test]
    fn test_extract_token_usage_partial_map() {
        let mut usage = std::collections::HashMap::new();
        usage.insert("total_tokens".to_string(), 200);
        let result = extract_token_usage(&usage);
        assert_eq!(
            result,
            Some(TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 0,
                total_tokens: 200,
            })
        );
    }

    #[test]
    fn test_extract_token_usage_negative_value_clamped() {
        let mut usage = std::collections::HashMap::new();
        usage.insert("prompt_tokens".to_string(), -5);
        usage.insert("completion_tokens".to_string(), 42);
        usage.insert("total_tokens".to_string(), 37);
        let result = extract_token_usage(&usage);
        assert_eq!(
            result,
            Some(TokenUsage {
                prompt_tokens: 0,
                completion_tokens: 42,
                total_tokens: 37,
            })
        );
    }

    #[test]
    fn test_accumulate_usage_sums_multiple_iterations() {
        let mut total = std::collections::HashMap::new();
        accumulate_usage(
            &mut total,
            &std::collections::HashMap::from([
                ("prompt_tokens".to_string(), 10),
                ("completion_tokens".to_string(), 15),
                ("total_tokens".to_string(), 25),
            ]),
        );
        accumulate_usage(
            &mut total,
            &std::collections::HashMap::from([
                ("prompt_tokens".to_string(), 7),
                ("completion_tokens".to_string(), 8),
                ("total_tokens".to_string(), 15),
            ]),
        );

        assert_eq!(extract_token_usage(&total).unwrap().prompt_tokens, 17);
        assert_eq!(extract_token_usage(&total).unwrap().completion_tokens, 23);
        assert_eq!(extract_token_usage(&total).unwrap().total_tokens, 40);
    }

    #[test]
    fn test_resolve_max_iterations_default_value() {
        // Assert that the default is 30 (not the old hardcoded 15)
        assert_eq!(SubagentManager::resolve_max_iterations(None, None), 30);
    }

    #[test]
    fn test_batch_spawn_request_max_iterations_field() {
        let request = BatchSpawnRequest {
            tasks: vec![SubAgentTask {
                id: "t1".to_string(),
                goal: "test".to_string(),
                context: None,
            }],
            max_iterations: Some(15),
        };
        assert_eq!(request.max_iterations, Some(15));

        let request_none = BatchSpawnRequest {
            tasks: vec![],
            max_iterations: None,
        };
        assert_eq!(request_none.max_iterations, None);
    }
}
