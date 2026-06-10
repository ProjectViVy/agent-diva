//! Subagent management for background tasks

use std::collections::HashMap;
use std::future::Future;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, Result};
use tokio::sync::{OwnedSemaphorePermit, RwLock, Semaphore};
use tokio::task::JoinHandle;
use tokio::time::timeout;
use tracing::{debug, error, info};
use uuid::Uuid;

use agent_diva_core::bus::{InboundMessage, MessageBus};
use agent_diva_core::utils::truncate;
use agent_diva_providers::base::{LLMProvider, Message};
use agent_diva_tooling::ToolRegistry;

use crate::agent_loop::context_retry::{prepare_budgeted_messages, should_retry_context_overflow};
use crate::context_budget::{
    provider_error_indicates_context_overflow, CompactionMode, ContextBudgetPolicy,
};
use crate::loop_guard::{
    LoopGuard, DEFAULT_REPEATED_FAILURE_THRESHOLD, DEFAULT_SUBAGENT_LOOP_TIMEOUT,
    DEFAULT_SUBAGENT_MAX_ITERATIONS,
};
use crate::subagent_policy::SubagentPolicy;
use crate::tool_assembly::ToolAssembly;
use crate::tool_config::builtin::BuiltInToolsConfig;
use crate::tool_config::network::NetworkToolConfig;
use agent_diva_core::config::MCPServerConfig;

pub const MAX_CONCURRENT_SUBAGENTS: usize = 8;
pub const DEFAULT_SUBAGENT_TIMEOUT_SECS: u64 = 300;
const DEFAULT_SUBAGENT_TIMEOUT: Duration = Duration::from_secs(DEFAULT_SUBAGENT_TIMEOUT_SECS);

#[derive(Debug, Clone)]
pub struct SubagentSpawnRequest {
    pub task: String,
    pub label: Option<String>,
    pub origin_channel: String,
    pub origin_chat_id: String,
    pub current_depth: usize,
    pub origin: String,
}

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
    running_tasks: Arc<tokio::sync::Mutex<HashMap<String, JoinHandle<()>>>>,
    subagent_policy: SubagentPolicy,
    concurrency_limit: Arc<Semaphore>,
    context_budget: ContextBudgetPolicy,
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
        subagent_policy: SubagentPolicy,
        context_budget: ContextBudgetPolicy,
    ) -> Self {
        let model = model.unwrap_or_else(|| provider.get_default_model());
        let exec_timeout = exec_timeout.unwrap_or(30);
        let effective_max_concurrent =
            Self::effective_max_concurrent(subagent_policy.max_concurrent);

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
            running_tasks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
            concurrency_limit: Arc::new(Semaphore::new(effective_max_concurrent)),
            subagent_policy,
            context_budget,
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
    pub async fn spawn(&self, request: SubagentSpawnRequest) -> Result<String> {
        self.ensure_depth_allowed(request.current_depth)?;
        let permit = self
            .concurrency_limit
            .clone()
            .try_acquire_owned()
            .map_err(|_| self.concurrent_limit_error())?;
        let task_id = Uuid::new_v4().to_string()[..8].to_string();
        let display_label = request.label.clone().unwrap_or_else(|| {
            if request.task.len() > 30 {
                let mut end = 30;
                while !request.task.is_char_boundary(end) {
                    end -= 1;
                }
                format!("{}...", &request.task[..end])
            } else {
                request.task.clone()
            }
        });

        let provider = Arc::clone(&self.provider);
        let workspace = self.workspace.clone();
        let bus = self.bus.clone();
        let model = self.model.clone();
        let builtin_tools = self.subagent_policy.builtin_tools(&self.builtin_tools);
        let parent_network_config = self.network_config.read().await.clone();
        let network_config = self.subagent_policy.network_config(&parent_network_config);
        let exec_timeout = self.exec_timeout;
        let restrict_to_workspace = self.restrict_to_workspace;
        let parent_mcp_servers = self.mcp_servers.read().await.clone();
        let mcp_servers = self.subagent_policy.mcp_servers(&parent_mcp_servers);
        let subagent_policy = self.subagent_policy.clone();
        let context_budget = self.context_budget.clone();
        let next_depth = request.current_depth + 1;
        let origin_channel = request.origin_channel.clone();
        let origin_chat_id = request.origin_chat_id.clone();
        let task = request.task.clone();
        let origin = request.origin.clone();

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
                subagent_policy,
                context_budget,
                next_depth,
                origin,
                permit,
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

        info!(
            "Spawned subagent [{}]: {} (depth={}, origin={})",
            task_id, display_label, next_depth, request.origin
        );
        Ok(format!(
            "Subagent [{}] started (id: {}). I'll notify you when it completes.",
            display_label, task_id
        ))
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
        subagent_policy: SubagentPolicy,
        context_budget: ContextBudgetPolicy,
        depth: usize,
        origin: String,
        _permit: OwnedSemaphorePermit,
    ) {
        info!(
            "Subagent [{}] starting task: {} (depth={}, origin={})",
            task_id, label, depth, origin
        );

        let result = Self::with_subagent_timeout(
            Self::execute_subagent_task(
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
                &subagent_policy,
                &context_budget,
            ),
            DEFAULT_SUBAGENT_TIMEOUT,
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
        subagent_policy: &SubagentPolicy,
        context_budget: &ContextBudgetPolicy,
    ) -> Result<String> {
        let tools: ToolRegistry = ToolAssembly::new(workspace.to_path_buf())
            .builtin(builtin_tools.clone())
            .with_network_config(network_config.clone())
            .with_exec_timeout(exec_timeout)
            .restrict_to_workspace(restrict_to_workspace)
            .mcp_servers(mcp_servers.clone())
            .build_subagent_registry(subagent_policy);
        let system_prompt = Self::build_subagent_prompt(task, workspace, subagent_policy);
        Self::execute_subagent_task_with_registry(
            task_id,
            task,
            provider,
            model,
            system_prompt,
            &tools,
            context_budget,
        )
        .await
    }

    async fn execute_subagent_task_with_registry(
        task_id: &str,
        task: &str,
        provider: &Arc<dyn LLMProvider>,
        model: &str,
        system_prompt: String,
        tools: &ToolRegistry,
        context_budget: &ContextBudgetPolicy,
    ) -> Result<String> {
        let mut messages = vec![
            Message::system(system_prompt),
            Message::user(task.to_string()),
        ];

        let mut iteration = 0;
        let mut loop_guard = LoopGuard::new(
            DEFAULT_SUBAGENT_MAX_ITERATIONS,
            DEFAULT_SUBAGENT_LOOP_TIMEOUT,
            DEFAULT_REPEATED_FAILURE_THRESHOLD,
        );
        let final_result = loop {
            iteration = match loop_guard.begin_iteration(iteration) {
                Ok(next_iteration) => next_iteration,
                Err(reason) => return Err(anyhow::anyhow!(reason.user_message())),
            };
            loop_guard
                .check_elapsed()
                .map_err(|reason| anyhow::anyhow!(reason.user_message()))?;

            let mut compaction_mode = CompactionMode::Normal;
            let mut overflow_retry_used = false;
            let tool_defs = tools.get_definitions();
            let response = loop {
                let prepared_request = prepare_budgeted_messages(
                    &messages,
                    &tool_defs,
                    context_budget,
                    compaction_mode,
                );
                let response = provider
                    .chat(
                        prepared_request.messages,
                        Some(tool_defs.clone()),
                        Some(model.to_string()),
                        2000,
                        0.7,
                    )
                    .await;
                match response {
                    Ok(response) => break response,
                    Err(error)
                        if should_retry_context_overflow(
                            context_budget,
                            &error,
                            overflow_retry_used,
                        ) =>
                    {
                        overflow_retry_used = true;
                        compaction_mode = CompactionMode::OverflowRecovery;
                        continue;
                    }
                    Err(error) if provider_error_indicates_context_overflow(&error) => {
                        return Err(anyhow!(context_budget.overflow_user_message()));
                    }
                    Err(error) => return Err(error.into()),
                }
            };

            if response.has_tool_calls() {
                // Add assistant message with tool calls
                messages.push(Message {
                    role: "assistant".to_string(),
                    content: response.content.clone().unwrap_or_default().into(),
                    name: None,
                    tool_call_id: None,
                    tool_calls: Some(response.tool_calls.clone()),
                    reasoning_content: response.reasoning_content.clone(),
                    thinking_blocks: None,
                });

                // Execute tools
                for tool_call in &response.tool_calls {
                    loop_guard
                        .check_elapsed()
                        .map_err(|reason| anyhow::anyhow!(reason.user_message()))?;
                    let args_json = serde_json::to_value(&tool_call.arguments)?;
                    let args_str = serde_json::to_string(&tool_call.arguments)?;
                    debug!(
                        "Subagent [{}] executing: {} with arguments: {}",
                        task_id, tool_call.name, args_str
                    );
                    let result = tools.execute(&tool_call.name, args_json).await;
                    if let Some(reason) = loop_guard.record_tool_result(
                        &tool_call.name,
                        &serde_json::json!(tool_call.arguments),
                        &result,
                    ) {
                        return Err(anyhow::anyhow!(reason.user_message()));
                    }
                    messages.push(Message::tool(result, tool_call.id.clone()));
                }
            } else {
                break response.content;
            }
        };

        Ok(final_result
            .unwrap_or_else(|| "Task completed but no final response was generated.".to_string()))
    }

    async fn with_subagent_timeout<T>(
        future: impl Future<Output = Result<T>>,
        timeout_duration: Duration,
    ) -> Result<T> {
        timeout(timeout_duration, future).await.map_err(|_| {
            anyhow!(
                "Subagent task timed out after {} seconds.",
                timeout_duration.as_secs()
            )
        })?
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
    fn build_subagent_prompt(task: &str, workspace: &Path, policy: &SubagentPolicy) -> String {
        let soul_summary = Self::build_identity_summary(workspace);
        let mut allowed = Vec::new();
        if policy.allow_filesystem {
            allowed.push("Read and write files in the workspace");
        }
        if policy.allow_shell {
            allowed.push("Execute shell commands");
        }
        if policy.allow_web_search {
            allowed.push("Search the web");
        }
        if policy.allow_web_fetch {
            allowed.push("Fetch web pages");
        }
        if policy.allow_mcp {
            allowed.push("Use enabled MCP tools");
        }
        let allowed_tools = if allowed.is_empty() {
            "- No delegated tools are enabled".to_string()
        } else {
            allowed
                .into_iter()
                .map(|item| format!("- {}", item))
                .collect::<Vec<_>>()
                .join("\n")
        };
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
{}
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
            allowed_tools,
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

    /// Get the number of currently running subagents
    pub async fn get_running_count(&self) -> usize {
        let tasks = self.running_tasks.lock().await;
        tasks.len()
    }

    pub fn subagent_policy(&self) -> &SubagentPolicy {
        &self.subagent_policy
    }

    fn ensure_depth_allowed(&self, current_depth: usize) -> Result<()> {
        if current_depth >= self.subagent_policy.max_depth {
            return Err(self.depth_limit_error(current_depth + 1));
        }
        Ok(())
    }

    fn concurrent_limit_error(&self) -> anyhow::Error {
        anyhow!(
            "Subagent spawn rejected: the concurrent subagent limit ({}) is already in use.",
            Self::effective_max_concurrent(self.subagent_policy.max_concurrent)
        )
    }

    fn depth_limit_error(&self, attempted_depth: usize) -> anyhow::Error {
        anyhow!(
            "Subagent spawn rejected: nesting depth {} exceeds the configured maximum of {}.",
            attempted_depth,
            self.subagent_policy.max_depth
        )
    }

    fn effective_max_concurrent(configured: usize) -> usize {
        configured.clamp(1, MAX_CONCURRENT_SUBAGENTS)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        SubagentManager, SubagentSpawnRequest, DEFAULT_SUBAGENT_TIMEOUT_SECS,
        MAX_CONCURRENT_SUBAGENTS,
    };
    use crate::subagent_policy::SubagentPolicy;
    use crate::tool_config::builtin::BuiltInToolsConfig;
    use crate::tool_config::network::{
        NetworkToolConfig, WebFetchRuntimeConfig, WebRuntimeConfig, WebSearchRuntimeConfig,
    };
    use crate::ContextBudgetPolicy;
    use agent_diva_core::bus::MessageBus;
    use agent_diva_core::config::MCPServerConfig;
    use agent_diva_providers::{
        LLMResponse, Message, ProviderError, ProviderResult, ToolCallRequest,
    };
    use agent_diva_tooling::{Tool, ToolRegistry};
    use async_trait::async_trait;
    use serde_json::json;
    use std::collections::HashMap;
    use std::future::pending;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio::sync::Notify;

    struct RepeatingToolProvider {
        args_sequence: Mutex<Vec<HashMap<String, serde_json::Value>>>,
    }
    struct BlockingProvider {
        notify: Arc<Notify>,
    }
    struct FailingTool;

    #[async_trait]
    impl agent_diva_providers::LLMProvider for RepeatingToolProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            let mut args_sequence = self.args_sequence.lock().unwrap();
            let arguments = args_sequence.remove(0);
            Ok(LLMResponse {
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
            })
        }

        async fn chat_stream(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<agent_diva_providers::ProviderEventStream> {
            Err(ProviderError::api_message(
                "chat_stream should not be used".to_string(),
            ))
        }

        fn get_default_model(&self) -> String {
            "test-model".to_string()
        }
    }

    #[async_trait]
    impl agent_diva_providers::LLMProvider for BlockingProvider {
        async fn chat(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<LLMResponse> {
            self.notify.notified().await;
            Ok(LLMResponse {
                content: Some("done".to_string()),
                tool_calls: Vec::new(),
                finish_reason: "stop".to_string(),
                usage: HashMap::new(),
                reasoning_content: None,
            })
        }

        async fn chat_stream(
            &self,
            _messages: Vec<Message>,
            _tools: Option<Vec<serde_json::Value>>,
            _model: Option<String>,
            _max_tokens: i32,
            _temperature: f64,
        ) -> ProviderResult<agent_diva_providers::ProviderEventStream> {
            Err(ProviderError::api_message(
                "chat_stream should not be used".to_string(),
            ))
        }

        fn get_default_model(&self) -> String {
            "test-model".to_string()
        }
    }

    #[async_trait]
    impl Tool for FailingTool {
        fn name(&self) -> &str {
            "fail_tool"
        }

        fn description(&self) -> &str {
            "Always fails"
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

        let prompt = SubagentManager::build_subagent_prompt(
            "analyze logs",
            temp.path(),
            &SubagentPolicy::default(),
        );
        assert!(prompt.contains("## Inherited Identity"));
        assert!(prompt.contains("### SOUL.md"));
        assert!(prompt.contains("### IDENTITY.md"));
        assert!(prompt.contains("### USER.md"));
    }

    #[test]
    fn test_build_subagent_prompt_fallback_without_identity_files() {
        let temp = tempfile::tempdir().unwrap();
        let prompt = SubagentManager::build_subagent_prompt(
            "analyze logs",
            temp.path(),
            &SubagentPolicy::default(),
        );
        assert!(prompt.contains("No persisted soul identity found"));
    }

    #[test]
    fn test_build_subagent_prompt_reflects_minimal_permissions() {
        let temp = tempfile::tempdir().unwrap();
        let prompt = SubagentManager::build_subagent_prompt(
            "inspect files",
            temp.path(),
            &SubagentPolicy::default(),
        );

        assert!(prompt.contains("Read and write files in the workspace"));
        assert!(prompt.contains("Execute shell commands"));
        assert!(!prompt.contains("Search the web"));
        assert!(!prompt.contains("Fetch web pages"));
        assert!(!prompt.contains("MCP"));
    }

    #[tokio::test]
    async fn test_execute_subagent_task_stops_on_repeated_failed_tool_call() {
        let provider: Arc<dyn agent_diva_providers::LLMProvider> =
            Arc::new(RepeatingToolProvider {
                args_sequence: Mutex::new(vec![
                    HashMap::from([("attempt".to_string(), json!(1))]),
                    HashMap::from([("attempt".to_string(), json!(1))]),
                    HashMap::from([("attempt".to_string(), json!(1))]),
                ]),
            });
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(FailingTool));

        let error = SubagentManager::execute_subagent_task_with_registry(
            "task-1",
            "inspect",
            &provider,
            "test-model",
            "system".to_string(),
            &registry,
            &ContextBudgetPolicy::default(),
        )
        .await
        .expect_err("subagent should stop on repeated tool failures");

        assert!(error.to_string().contains("repeated failures"));
        assert!(error.to_string().contains("fail_tool"));
    }

    #[test]
    fn test_policy_minimizes_network_and_mcp_for_subagent() {
        let policy = SubagentPolicy::default();
        let parent_network = NetworkToolConfig {
            web: WebRuntimeConfig {
                search: WebSearchRuntimeConfig {
                    provider: "bocha".to_string(),
                    enabled: true,
                    api_key: Some("secret-key".to_string()),
                    max_results: 5,
                },
                fetch: WebFetchRuntimeConfig { enabled: true },
            },
        };
        let trimmed_network = policy.network_config(&parent_network);
        let mut parent_mcp = HashMap::new();
        parent_mcp.insert(
            "demo".to_string(),
            MCPServerConfig {
                url: "http://127.0.0.1:8080".to_string(),
                ..MCPServerConfig::default()
            },
        );

        assert!(!trimmed_network.web.search.enabled);
        assert!(trimmed_network.web.search.api_key.is_none());
        assert!(!trimmed_network.web.fetch.enabled);
        assert!(policy.mcp_servers(&parent_mcp).is_empty());
    }

    #[tokio::test]
    async fn test_subagent_manager_rejects_when_concurrency_limit_reached() {
        let notify = Arc::new(Notify::new());
        let provider: Arc<dyn agent_diva_providers::LLMProvider> = Arc::new(BlockingProvider {
            notify: notify.clone(),
        });
        let manager = SubagentManager::new(
            provider,
            tempfile::tempdir().unwrap().path().to_path_buf(),
            MessageBus::new(),
            Some("test-model".to_string()),
            BuiltInToolsConfig::default(),
            NetworkToolConfig::default(),
            Some(5),
            false,
            HashMap::new(),
            SubagentPolicy {
                max_concurrent: 1,
                ..SubagentPolicy::default()
            },
            ContextBudgetPolicy::default(),
        );

        let first = manager
            .spawn(SubagentSpawnRequest {
                task: "hold".to_string(),
                label: Some("hold".to_string()),
                origin_channel: "cli".to_string(),
                origin_chat_id: "direct".to_string(),
                current_depth: 0,
                origin: "test".to_string(),
            })
            .await
            .expect("first spawn should succeed");
        assert!(first.contains("started"));

        let err = manager
            .spawn(SubagentSpawnRequest {
                task: "second".to_string(),
                label: Some("second".to_string()),
                origin_channel: "cli".to_string(),
                origin_chat_id: "direct".to_string(),
                current_depth: 0,
                origin: "test".to_string(),
            })
            .await
            .expect_err("second spawn should be rejected");
        assert!(err.to_string().contains("concurrent subagent limit"));

        notify.notify_waiters();
    }

    #[tokio::test]
    async fn test_subagent_manager_rejects_when_depth_exceeded() {
        let provider: Arc<dyn agent_diva_providers::LLMProvider> =
            Arc::new(RepeatingToolProvider {
                args_sequence: Mutex::new(vec![HashMap::from([("attempt".to_string(), json!(1))])]),
            });
        let manager = SubagentManager::new(
            provider,
            tempfile::tempdir().unwrap().path().to_path_buf(),
            MessageBus::new(),
            Some("test-model".to_string()),
            BuiltInToolsConfig::default(),
            NetworkToolConfig::default(),
            Some(5),
            false,
            HashMap::new(),
            SubagentPolicy {
                max_depth: 1,
                ..SubagentPolicy::default()
            },
            ContextBudgetPolicy::default(),
        );

        let err = manager
            .spawn(SubagentSpawnRequest {
                task: "too deep".to_string(),
                label: Some("too-deep".to_string()),
                origin_channel: "cli".to_string(),
                origin_chat_id: "direct".to_string(),
                current_depth: 1,
                origin: "test".to_string(),
            })
            .await
            .expect_err("depth violation should be rejected");
        assert!(err.to_string().contains("nesting depth"));
    }

    #[tokio::test]
    async fn test_subagent_timeout_helper_returns_error() {
        let err = SubagentManager::with_subagent_timeout(
            pending::<anyhow::Result<()>>(),
            Duration::from_millis(10),
        )
        .await
        .expect_err("pending subagent future should time out");

        assert!(err.to_string().contains("timed out"));
    }

    #[test]
    fn test_subagent_default_timeout_is_300_seconds() {
        assert_eq!(DEFAULT_SUBAGENT_TIMEOUT_SECS, 300);
    }

    #[test]
    fn test_subagent_effective_concurrency_is_hard_capped() {
        assert_eq!(
            SubagentManager::effective_max_concurrent(MAX_CONCURRENT_SUBAGENTS + 1),
            MAX_CONCURRENT_SUBAGENTS
        );
        assert_eq!(SubagentManager::effective_max_concurrent(0), 1);
    }
}
