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
use agent_diva_core::config::schema::{BatchSpawnRequest, MaskConfig, SubAgentResult, SubAgentStatus, ToolLimits};
use agent_diva_core::utils::truncate;
use agent_diva_providers::base::{LLMProvider, Message};
use agent_diva_tooling::ToolRegistry;

use crate::mask::tool_policy::ToolPolicy;

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
    parent_tool_limits: ToolLimits,
    running_tasks: Arc<tokio::sync::Mutex<HashMap<String, JoinHandle<()>>>>,
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
            running_tasks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
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
    pub fn resolve_max_iterations(
        spawn_limit: Option<u32>,
        mask: Option<&MaskConfig>,
    ) -> u32 {
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
        let display_label = label.unwrap_or_else(|| {
            if task.len() > 30 {
                let mut end = 30;
                while !task.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &task[..end])
            } else {
                task.clone()
            }
        });

        let provider = Arc::clone(&self.provider);
        let workspace = self.workspace.clone();
        let bus = self.bus.clone();
        let model = self.model.clone();
        let builtin_tools = self.builtin_tools.clone();
        let network_config = self.network_config.read().await.clone();
        let exec_timeout = self.exec_timeout;
        let restrict_to_workspace = self.restrict_to_workspace;
        let mcp_servers = self.mcp_servers.read().await.clone();

        let task_id_clone = task_id.clone();
        let display_label_clone = display_label.clone();
        let running_tasks = Arc::clone(&self.running_tasks);

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
        let model = self.model.clone();
        let builtin_tools = self.builtin_tools.clone();
        let network_config = self.network_config.read().await.clone();
        let exec_timeout = self.exec_timeout;
        let restrict_to_workspace = self.restrict_to_workspace;
        let mcp_servers = self.mcp_servers.read().await.clone();

        let mut join_set = JoinSet::new();
        let mut tasks = VecDeque::from(request.tasks);
        let max_concurrent = MAX_CONCURRENT_SUBAGENTS.max(1);

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
                    )
                    .await
                });
            }
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
            ),
        )
        .await;

        let elapsed_ms = start.elapsed().as_millis() as u64;

        match exec_result {
            Ok(Ok((summary, tool_call_count, tool_trace))) => {
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
                    token_usage: None,
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
                    summary: Some(format!(
                        "Task timed out after {}s",
                        exec_timeout
                    )),
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
    /// Returns `(summary, tool_call_count, tool_trace)` on success.
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
    ) -> Result<(String, u32, Vec<String>)> {
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

        let max_iterations = 15;
        let mut iteration = 0;
        let mut final_result: Option<String> = None;
        let mut tool_call_count: u32 = 0;
        let mut tool_trace: Vec<String> = Vec::new();

        while iteration < max_iterations {
            iteration += 1;

            let response = provider
                .chat(
                    messages.clone(),
                    Some(tools.get_definitions()),
                    Some(model.to_string()),
                    2000,
                    0.7,
                )
                .await?;

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
                    let result = tools.execute(&tool_call.name, args_json).await;
                    messages.push(Message::tool(result, tool_call.id.clone()));
                }
            } else {
                final_result = response.content;
                break;
            }
        }

        let summary = final_result
            .unwrap_or_else(|| "Task completed but no final response was generated.".to_string());
        Ok((summary, tool_call_count, tool_trace))
    }

    /// Execute the subagent task with LLM and tools
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
    ) -> Result<String> {
        let tools: ToolRegistry = ToolAssembly::new(workspace.to_path_buf())
            .builtin(builtin_tools.clone())
            .with_network_config(network_config.clone())
            .with_exec_timeout(exec_timeout)
            .restrict_to_workspace(restrict_to_workspace)
            .mcp_servers(mcp_servers.clone())
            .build_subagent_registry();

        // Build messages with subagent-specific prompt
        let system_prompt = Self::build_subagent_prompt(task, workspace);
        let mut messages = vec![
            Message::system(system_prompt),
            Message::user(task.to_string()),
        ];

        // Run agent loop (limited iterations)
        let max_iterations = 15;
        let mut iteration = 0;
        let mut final_result: Option<String> = None;

        while iteration < max_iterations {
            iteration += 1;

            let response = provider
                .chat(
                    messages.clone(),
                    Some(tools.get_definitions()),
                    Some(model.to_string()),
                    2000,
                    0.7,
                )
                .await?;

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
                    let result = tools.execute(&tool_call.name, args_json).await;
                    messages.push(Message::tool(result, tool_call.id.clone()));
                }
            } else {
                final_result = response.content;
                break;
            }
        }

        Ok(final_result
            .unwrap_or_else(|| "Task completed but no final response was generated.".to_string()))
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
    fn build_subagent_prompt(task: &str, workspace: &Path) -> String {
        let soul_summary = Self::build_identity_summary(workspace);
        format!(
            r#"# Subagent

You are a subagent spawned by the main agent to complete a specific task.

## Your Task
{}

## Inherited Identity
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
- Spawn other subagents
- Access the main agent's conversation history

## Workspace
Your workspace is at: {}

When you have completed the task, provide a clear summary of your findings or actions."#,
            task,
            soul_summary,
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

    fn build_identity_summary(workspace: &Path) -> String {
        let mut sections = Vec::new();
        for file in ["SOUL.md", "IDENTITY.md", "USER.md"] {
            let path = workspace.join(file);
            let Ok(raw) = std::fs::read_to_string(path) else {
                continue;
            };
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                continue;
            }
            let content = if trimmed.chars().count() > 800 {
                truncate(trimmed, 3200)
            } else {
                trimmed.to_string()
            };
            sections.push(format!("### {}\n{}", file, content));
        }

        if sections.is_empty() {
            "No persisted soul identity found. Follow the task faithfully, remain concise, and preserve user intent.".to_string()
        } else {
            sections.join("\n\n")
        }
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
        )
        .await;

        let (final_result, status) = match result {
            Ok(content) => {
                info!("Subagent [{}] completed successfully", task_id);
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
    pub(crate) fn builtin_tools_for_test(&self) -> &BuiltInToolsConfig {
        &self.builtin_tools
    }
}

#[cfg(test)]
mod tests {
    use super::SubagentManager;
    use agent_diva_core::config::schema::{
        BatchSpawnRequest, MaskConfig, SubAgentStatus, SubAgentTask, SubagentDefaults,
    };

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
        let result = SubagentManager::resolve_model(
            Some("explicit-model"),
            Some(&mask),
            "global-default",
        );
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
    fn test_build_subagent_prompt_includes_identity_summary() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("SOUL.md"), "# Soul\n\nKeep concise.").unwrap();
        std::fs::write(temp.path().join("IDENTITY.md"), "# Identity\n\nAgent Diva.").unwrap();
        std::fs::write(
            temp.path().join("USER.md"),
            "# User\n\nPrefer direct replies.",
        )
        .unwrap();

        let prompt = SubagentManager::build_subagent_prompt("analyze logs", temp.path());
        assert!(prompt.contains("## Inherited Identity"));
        assert!(prompt.contains("### SOUL.md"));
        assert!(prompt.contains("### IDENTITY.md"));
        assert!(prompt.contains("### USER.md"));
    }

    #[test]
    fn test_build_subagent_prompt_fallback_without_identity_files() {
        let temp = tempfile::tempdir().unwrap();
        let prompt = SubagentManager::build_subagent_prompt("analyze logs", temp.path());
        assert!(prompt.contains("No persisted soul identity found"));
    }

    #[test]
    fn test_build_subagent_prompt_omits_mentle_routing() {
        let temp = tempfile::tempdir().unwrap();
        let prompt = SubagentManager::build_subagent_prompt("analyze logs", temp.path());

        assert!(!prompt.contains("L2 Palace Memory"));
        assert!(!prompt.contains("memtle_status"));
        assert!(!prompt.contains("memtle_search"));
    }

    // ── build_isolated_subagent_prompt tests ──────────────────────────────

    #[test]
    fn test_isolated_prompt_has_no_soul() {
        let temp = tempfile::tempdir().unwrap();
        std::fs::write(temp.path().join("SOUL.md"), "# Soul\n\nYou are Diva.").unwrap();

        let prompt =
            SubagentManager::build_isolated_subagent_prompt("summarize file", temp.path());
        assert!(!prompt.contains("SOUL.md"));
        assert!(!prompt.contains("You are Diva"));
        assert!(!prompt.contains("Inherited Identity"));
        assert!(prompt.contains("no personality"));
        assert!(prompt.contains("summarize file"));
    }

    #[test]
    fn test_isolated_prompt_includes_workspace_path() {
        let temp = tempfile::tempdir().unwrap();
        let prompt =
            SubagentManager::build_isolated_subagent_prompt("task", temp.path());
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
        };
        assert_eq!(request.tasks.len(), 2);
        assert_eq!(request.tasks[0].id, "t1");
        assert_eq!(request.tasks[1].context.as_deref(), Some("extra info"));
    }

    #[test]
    fn batch_spawn_empty_tasks() {
        let request = BatchSpawnRequest { tasks: vec![] };
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
            token_usage: None,
            tool_trace: Some(vec!["read_file".to_string(), "write_file".to_string()]),
        };
        assert_eq!(result.task_id, "batch-task-1");
        assert_eq!(result.status, SubAgentStatus::Ok);
        assert_eq!(result.tool_call_count, 3);
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
}
