//! Subagent management for background tasks

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{debug, error, info};
use uuid::Uuid;

use agent_diva_core::bus::{InboundMessage, MessageBus};
use agent_diva_core::utils::truncate;
use agent_diva_providers::base::{LLMProvider, Message};
use agent_diva_tools::registry::ToolRegistry;
use agent_diva_tools::{
    filesystem::{ListDirTool, ReadFileTool, WriteFileTool},
    shell::ExecTool,
    web::{WebFetchTool, WebSearchTool},
};

use crate::tool_config::network::NetworkToolConfig;

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
    network_config: Arc<RwLock<NetworkToolConfig>>,
    exec_timeout: u64,
    restrict_to_workspace: bool,
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
        network_config: NetworkToolConfig,
        exec_timeout: Option<u64>,
        restrict_to_workspace: bool,
    ) -> Self {
        let model = model.unwrap_or_else(|| provider.get_default_model());
        let exec_timeout = exec_timeout.unwrap_or(30);

        Self {
            provider,
            workspace,
            bus,
            model,
            network_config: Arc::new(RwLock::new(network_config)),
            exec_timeout,
            restrict_to_workspace,
            running_tasks: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    pub async fn update_network_config(&self, network_config: NetworkToolConfig) {
        let mut guard = self.network_config.write().await;
        *guard = network_config;
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
        let network_config = self.network_config.read().await.clone();
        let exec_timeout = self.exec_timeout;
        let restrict_to_workspace = self.restrict_to_workspace;

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
                network_config,
                exec_timeout,
                restrict_to_workspace,
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
        network_config: NetworkToolConfig,
        exec_timeout: u64,
        restrict_to_workspace: bool,
    ) {
        info!("Subagent [{}] starting task: {}", task_id, label);

        let result = Self::execute_subagent_task(
            &task_id,
            &task,
            &provider,
            &workspace,
            &model,
            &network_config,
            exec_timeout,
            restrict_to_workspace,
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
        network_config: &NetworkToolConfig,
        _exec_timeout: u64,
        restrict_to_workspace: bool,
    ) -> Result<String> {
        // Build subagent tools (no message tool, no spawn tool)
        let mut tools = ToolRegistry::new();
        let allowed_dir = if restrict_to_workspace {
            Some(workspace.to_path_buf())
        } else {
            None
        };

        tools.register(Arc::new(ReadFileTool::new(allowed_dir.clone())));
        tools.register(Arc::new(WriteFileTool::new(allowed_dir.clone())));
        tools.register(Arc::new(ListDirTool::new(allowed_dir)));
        tools.register(Arc::new(ExecTool::new()));
        if network_config.web.search.enabled {
            tools.register(Arc::new(WebSearchTool::with_provider_and_max_results(
                network_config.web.search.provider.clone(),
                network_config.web.search.api_key.clone(),
                network_config.web.search.normalized_max_results(),
            )));
        }
        if network_config.web.fetch.enabled {
            tools.register(Arc::new(WebFetchTool::new()));
        }

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
                    content: response.content.clone().unwrap_or_default(),
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
        let msg = InboundMessage::new(
            origin_channel,
            "subagent",
            origin_chat_id,
            announce_content,
        );

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
}

#[cfg(test)]
mod tests {
    use super::SubagentManager;

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
}
