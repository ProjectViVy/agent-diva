//! Agent loop: the core processing engine

use futures::StreamExt;
use agent_diva_core::bus::{InboundMessage, MessageBus, OutboundMessage, AgentEvent};
use agent_diva_core::config::MCPServerConfig;
use agent_diva_core::session::SessionManager;
use agent_diva_providers::{LLMProvider, LLMResponse, LLMStreamEvent};
use agent_diva_tools::{
    load_mcp_tools, EditFileTool, ExecTool, ListDirTool, ReadFileTool, SpawnTool, ToolError,
    ToolRegistry, WebFetchTool, WebSearchTool, WriteFileTool,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::context::ContextBuilder;
use crate::subagent::SubagentManager;

/// Configuration for tool setup
#[derive(Clone)]
pub struct ToolConfig {
    /// Brave API key for web search
    pub brave_api_key: Option<String>,
    /// Shell execution timeout in seconds
    pub exec_timeout: u64,
    /// Whether to restrict file access to workspace
    pub restrict_to_workspace: bool,
    /// Configured MCP servers
    pub mcp_servers: HashMap<String, MCPServerConfig>,
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            brave_api_key: None,
            exec_timeout: 60,
            restrict_to_workspace: false,
            mcp_servers: HashMap::new(),
        }
    }
}

/// The agent loop is the core processing engine
pub struct AgentLoop {
    bus: MessageBus,
    provider: Arc<dyn LLMProvider>,
    #[allow(dead_code)]
    workspace: PathBuf,
    model: String,
    max_iterations: usize,
    context: ContextBuilder,
    sessions: SessionManager,
    tools: ToolRegistry,
    subagent_manager: Arc<SubagentManager>,
}

impl AgentLoop {
    /// Create a new agent loop
    pub fn new(
        bus: MessageBus,
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        model: Option<String>,
        max_iterations: Option<usize>,
    ) -> Self {
        let model = model.unwrap_or_else(|| provider.get_default_model());
        let context = ContextBuilder::new(workspace.clone());
        let sessions = SessionManager::new(workspace.clone());
        let tools = ToolRegistry::new();

        let subagent_manager = Arc::new(SubagentManager::new(
            provider.clone(),
            workspace.clone(),
            bus.clone(),
            Some(model.clone()),
            None,
            None,
            false,
        ));

        Self {
            bus,
            provider,
            workspace,
            model,
            max_iterations: max_iterations.unwrap_or(20),
            context,
            sessions,
            tools,
            subagent_manager,
        }
    }

    /// Create a new agent loop with tool configuration
    pub fn with_tools(
        bus: MessageBus,
        provider: Arc<dyn LLMProvider>,
        workspace: PathBuf,
        model: Option<String>,
        max_iterations: Option<usize>,
        tool_config: ToolConfig,
    ) -> Self {
        let model = model.unwrap_or_else(|| provider.get_default_model());
        let context = ContextBuilder::new(workspace.clone());
        let sessions = SessionManager::new(workspace.clone());
        let mut tools = ToolRegistry::new();

        let subagent_manager = Arc::new(SubagentManager::new(
            provider.clone(),
            workspace.clone(),
            bus.clone(),
            Some(model.clone()),
            tool_config.brave_api_key.clone(),
            Some(tool_config.exec_timeout),
            tool_config.restrict_to_workspace,
        ));

        // Register spawn tool
        let sm = subagent_manager.clone();
        tools.register(Arc::new(SpawnTool::new(move |task, label, channel, chat_id| {
            let sm = sm.clone();
            async move {
                sm.spawn(task, label, channel, chat_id)
                    .await
                    .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
            }
        })));

        // Register file system tools
        let allowed_dir = if tool_config.restrict_to_workspace {
            Some(workspace.clone())
        } else {
            None
        };
        tools.register(Arc::new(ReadFileTool::new(allowed_dir.clone())));
        tools.register(Arc::new(WriteFileTool::new(allowed_dir.clone())));
        tools.register(Arc::new(EditFileTool::new(allowed_dir.clone())));
        tools.register(Arc::new(ListDirTool::new(allowed_dir)));

        // Register shell tool
        tools.register(Arc::new(ExecTool::with_config(
            tool_config.exec_timeout,
            Some(workspace.clone()),
            tool_config.restrict_to_workspace,
        )));

        // Register web tools
        tools.register(Arc::new(WebSearchTool::new(tool_config.brave_api_key)));
        tools.register(Arc::new(WebFetchTool::new()));

        // Register MCP tools discovered from configured servers
        for mcp_tool in load_mcp_tools(&tool_config.mcp_servers) {
            tools.register(mcp_tool);
        }

        Self {
            bus,
            provider,
            workspace,
            model,
            max_iterations: max_iterations.unwrap_or(20),
            context,
            sessions,
            tools,
            subagent_manager,
        }
    }

    /// Register default tools (for use after construction)
    pub fn register_default_tools(&mut self, tool_config: ToolConfig) {
        // Register spawn tool
        let sm = self.subagent_manager.clone();
        self.tools
            .register(Arc::new(SpawnTool::new(move |task, label, channel, chat_id| {
                let sm = sm.clone();
                async move {
                    sm.spawn(task, label, channel, chat_id)
                        .await
                        .map_err(|e| ToolError::ExecutionFailed(e.to_string()))
                }
            })));

        // Register file system tools
        let allowed_dir = if tool_config.restrict_to_workspace {
            Some(self.workspace.clone())
        } else {
            None
        };
        self.tools
            .register(Arc::new(ReadFileTool::new(allowed_dir.clone())));
        self.tools
            .register(Arc::new(WriteFileTool::new(allowed_dir.clone())));
        self.tools
            .register(Arc::new(EditFileTool::new(allowed_dir.clone())));
        self.tools.register(Arc::new(ListDirTool::new(allowed_dir)));

        // Register shell tool
        self.tools.register(Arc::new(ExecTool::with_config(
            tool_config.exec_timeout,
            Some(self.workspace.clone()),
            tool_config.restrict_to_workspace,
        )));

        // Register web tools
        self.tools
            .register(Arc::new(WebSearchTool::new(tool_config.brave_api_key)));
        self.tools.register(Arc::new(WebFetchTool::new()));

        // Register MCP tools discovered from configured servers
        for mcp_tool in load_mcp_tools(&tool_config.mcp_servers) {
            self.tools.register(mcp_tool);
        }
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
            // Try to receive a message with timeout
            match tokio::time::timeout(std::time::Duration::from_secs(1), inbound_rx.recv()).await {
                Ok(Some(msg)) => {
                    debug!("Received message from {}:{}", msg.channel, msg.chat_id);
                    match self.process_inbound_message(msg, None).await {
                        Ok(Some(response)) => {
                            if let Err(e) = self.bus.publish_outbound(response) {
                                error!("Failed to publish response: {}", e);
                            }
                        }
                        Ok(None) => {
                            debug!("No response needed");
                        }
                        Err(e) => {
                            error!("Error processing message: {}", e);
                        }
                    }
                }
                Ok(None) => {
                    // Channel closed
                    info!("Message bus closed, stopping agent loop");
                    break;
                }
                Err(_) => {
                    // Timeout, continue
                    continue;
                }
            }
        }

        info!("Agent loop stopped");
        Ok(())
    }

    /// Process a single inbound message
    pub async fn process_inbound_message(
        &mut self,
        msg: InboundMessage,
        event_tx: Option<&mpsc::UnboundedSender<AgentEvent>>,
    ) -> Result<Option<OutboundMessage>, Box<dyn std::error::Error>> {
        // Use the default model from the current provider
        let model_to_use = self.provider.get_default_model();
        
        let preview = if msg.content.chars().count() > 80 {
            format!("{}...", msg.content.chars().take(80).collect::<String>())
        } else {
            msg.content.clone()
        };
        println!(
            "Processing message from {}:{}: {} (model: {})",
            msg.channel, msg.sender_id, preview, model_to_use
        );

        // Get or create session
        let session_key = format!("{}:{}", msg.channel, msg.chat_id);
        let session = self.sessions.get_or_create(&session_key);

        // Build initial messages
        let history = session.get_history(50); // Last 50 messages
        let mut messages = self.context.build_messages(
            history,
            msg.content.clone(),
            Some(&msg.channel),
            Some(&msg.chat_id),
        );

        // Agent loop
        let mut iteration = 0;
        let mut final_content: Option<String> = None;
        let mut final_reasoning: Option<String> = None;

        while iteration < self.max_iterations {
            iteration += 1;
            debug!("Agent iteration {}/{}", iteration, self.max_iterations);
            let event = AgentEvent::IterationStarted {
                index: iteration,
                max_iterations: self.max_iterations,
            };
            if let Some(tx) = event_tx {
                let _ = tx.send(event.clone());
            }
            let _ = self.bus.publish_event(msg.channel.clone(), msg.chat_id.clone(), event);

            // Call LLM (streaming when provider supports it)
            let tool_defs = self.tools.get_definitions();
            let mut stream = self
                .provider
                .chat_stream(
                    messages.clone(),
                    if !tool_defs.is_empty() {
                        Some(tool_defs)
                    } else {
                        None
                    },
                    Some(model_to_use.clone()),
                    4096,
                    0.7,
                )
                .await?;
            let mut streamed_content = String::new();
            let mut streamed_reasoning = String::new();
            let mut response: Option<LLMResponse> = None;
            while let Some(stream_event) = stream.next().await {
                match stream_event? {
                    LLMStreamEvent::TextDelta(delta) => {
                        streamed_content.push_str(&delta);
                        let event = AgentEvent::AssistantDelta { text: delta };
                        if let Some(tx) = event_tx {
                            let _ = tx.send(event.clone());
                        }
                        let _ = self.bus.publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
                    }
                    LLMStreamEvent::ReasoningDelta(delta) => {
                        debug!("Stream ReasoningDelta: {:?}", delta);
                        streamed_reasoning.push_str(&delta);
                        let event = AgentEvent::ReasoningDelta { text: delta };
                        if let Some(tx) = event_tx {
                            let _ = tx.send(event.clone());
                        }
                        let _ = self.bus.publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
                    }
                    LLMStreamEvent::ToolCallDelta { name, arguments_delta, .. } => {
                        if let Some(delta) = arguments_delta {
                            let event = AgentEvent::ToolCallDelta {
                                name,
                                args_delta: delta,
                            };
                            if let Some(tx) = event_tx {
                                let _ = tx.send(event.clone());
                            }
                            let _ = self.bus.publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
                        }
                    }
                    LLMStreamEvent::Completed(done) => {
                        response = Some(done);
                        break;
                    }
                }
            }
            let response = response.unwrap_or_else(|| LLMResponse {
                content: if streamed_content.is_empty() {
                    None
                } else {
                    Some(streamed_content)
                },
                tool_calls: Vec::new(),
                finish_reason: "stop".to_string(),
                usage: std::collections::HashMap::new(),
                reasoning_content: if streamed_reasoning.is_empty() {
                    None
                } else {
                    Some(streamed_reasoning)
                },
            });

            // Handle tool calls
            if response.has_tool_calls() {
                info!("LLM requested {} tool calls", response.tool_calls.len());

                // Add assistant message with tool calls
                self.context.add_assistant_message(
                    &mut messages,
                    response.content.clone(),
                    Some(response.tool_calls.clone()),
                    response.reasoning_content.clone(),
                );

                // Execute tools
                for tool_call in &response.tool_calls {
                    let args_str = serde_json::to_string(&tool_call.arguments).unwrap_or_default();
                    let preview = if args_str.chars().count() > 200 {
                        format!("{}...", args_str.chars().take(200).collect::<String>())
                    } else {
                        args_str.clone()
                    };
                    info!("Tool call: {}({})", tool_call.name, preview);
                    let event = AgentEvent::ToolCallStarted {
                        name: tool_call.name.clone(),
                        args_preview: preview.clone(),
                        call_id: tool_call.id.clone(),
                    };
                    if let Some(tx) = event_tx {
                        let _ = tx.send(event.clone());
                    }
                    let _ = self.bus.publish_event(msg.channel.clone(), msg.chat_id.clone(), event);

                    // Convert HashMap to Value for execute
                    let params_value = serde_json::to_value(&tool_call.arguments)
                        .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                    let result = self.tools.execute(&tool_call.name, params_value).await;
                    let event = AgentEvent::ToolCallFinished {
                        name: tool_call.name.clone(),
                        is_error: result.starts_with("Error"),
                        result: result.clone(),
                        call_id: tool_call.id.clone(),
                    };
                    if let Some(tx) = event_tx {
                        let _ = tx.send(event.clone());
                    }
                    let _ = self.bus.publish_event(msg.channel.clone(), msg.chat_id.clone(), event);
                    self.context.add_tool_result(
                        &mut messages,
                        tool_call.id.clone(),
                        tool_call.name.clone(),
                        result,
                    );
                }
            } else {
                // No tool calls, we're done
                final_content = response.content;
                final_reasoning = response.reasoning_content;
                break;
            }
        }

        let final_content = final_content.unwrap_or_else(|| {
            "I've completed processing but have no response to give.".to_string()
        });

        // Log response preview - use char indices to handle multi-byte UTF-8 characters safely
        let preview = if final_content.chars().count() > 120 {
            format!("{}...", final_content.chars().take(120).collect::<String>())
        } else {
            final_content.clone()
        };
        info!("Response to {}:{}: {}", msg.channel, msg.sender_id, preview);
        let event = AgentEvent::FinalResponse {
            content: final_content.clone(),
        };
        if let Some(tx) = event_tx {
            let _ = tx.send(event.clone());
        }
        let _ = self.bus.publish_event(msg.channel.clone(), msg.chat_id.clone(), event);

        // Save to session (ignore errors for now - session is auto-saved on drop)
        let session = self.sessions.get_or_create(&session_key);
        session.add_message("user", &msg.content);
        session.add_message("assistant", &final_content);

        // Extract reply_to from metadata if available (critical for platforms like QQ)
        let reply_to = msg
            .metadata
            .get("message_id")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(Some(OutboundMessage {
            channel: msg.channel,
            chat_id: msg.chat_id,
            content: final_content,
            reply_to,
            media: vec![],
            reasoning_content: final_reasoning,
            metadata: msg.metadata,
        }))
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_providers::LiteLLMClient;

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
}
