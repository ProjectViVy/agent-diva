//! MCP tools using rust-mcp-sdk.
//!
//! This module provides MCP tool integration using the official rust-mcp-sdk,
//! similar to nanobot's MCP implementation pattern.

use crate::sanitize::sanitize_for_json;
use agent_diva_core::config::MCPServerConfig;
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use rust_mcp_sdk::{
    mcp_client::{client_runtime, ClientHandler, ClientRuntime, McpClientOptions},
    schema::{
        CallToolRequestParams, ClientCapabilities, Implementation, InitializeRequestParams,
        LATEST_PROTOCOL_VERSION,
    },
    ClientSseTransport, ClientSseTransportOptions, McpClient, StdioTransport, ToMcpClientHandler,
    TransportOptions,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::warn;

/// Sanitize a JSON value by recursively cleaning all string values.
/// This removes control characters and ANSI sequences from strings within the JSON.
fn sanitize_json_strings(value: &mut Value) {
    match value {
        Value::String(s) => {
            *s = sanitize_for_json(s);
        }
        Value::Array(arr) => {
            for item in arr {
                sanitize_json_strings(item);
            }
        }
        Value::Object(map) => {
            for v in map.values_mut() {
                sanitize_json_strings(v);
            }
        }
        _ => {}
    }
}

/// Default timeout for MCP operations in seconds.
#[allow(dead_code)]
const DEFAULT_TIMEOUT_SECS: u64 = 30;

// ============================================================================
// Error Types
// ============================================================================

/// Error type for MCP operations.
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Failed to start MCP process: {0}")]
    ProcessStart(String),

    #[error("Failed to connect to MCP server: {0}")]
    ConnectionFailed(String),

    #[error("MCP request timed out")]
    Timeout,

    #[error("MCP SDK error: {0}")]
    Sdk(String),

    #[error("MCP server error: {0}")]
    Server(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

// ============================================================================
// Data Types
// ============================================================================

/// Discovered tool from an MCP server.
#[derive(Debug, Clone)]
pub struct DiscoveredTool {
    pub original_name: String,
    pub description: String,
    pub input_schema: Value,
}

/// MCP client wrapper that manages connection and tool calls.
pub struct McpClientWrapper {
    server_name: String,
    client: Arc<ClientRuntime>,
    tool_timeout: u64,
}

impl std::fmt::Debug for McpClientWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpClientWrapper")
            .field("server_name", &self.server_name)
            .field("tool_timeout", &self.tool_timeout)
            .finish()
    }
}

impl McpClientWrapper {
    /// Create a new MCP client for a stdio-based server.
    pub async fn new_stdio(server_name: &str, config: &MCPServerConfig) -> Result<Self, McpError> {
        let command_str = config.command.trim();
        if command_str.is_empty() {
            return Err(McpError::Config(
                "command is required for stdio transport".to_string(),
            ));
        }

        // Resolve command path on Windows
        let resolved_command = if cfg!(target_os = "windows") {
            which::which(command_str)
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| command_str.to_string())
        } else {
            command_str.to_string()
        };

        // Create transport with server launch
        let transport = StdioTransport::create_with_server_launch(
            &resolved_command,
            config.args.to_vec(),
            if config.env.is_empty() {
                None
            } else {
                Some(config.env.clone())
            },
            TransportOptions::default(),
        )
        .map_err(|e| McpError::ProcessStart(e.to_string()))?;

        Self::create_client(server_name, transport, config.tool_timeout).await
    }

    /// Create a new MCP client for an HTTP-based server (SSE transport).
    pub async fn new_sse(server_name: &str, config: &MCPServerConfig) -> Result<Self, McpError> {
        if config.url.trim().is_empty() {
            return Err(McpError::Config(
                "url is required for HTTP transport".to_string(),
            ));
        }

        // Use SSE transport for HTTP
        let transport = ClientSseTransport::new(&config.url, ClientSseTransportOptions::default())
            .map_err(|e| McpError::ConnectionFailed(e.to_string()))?;

        Self::create_client(server_name, transport, config.tool_timeout).await
    }

    async fn create_client<T>(
        server_name: &str,
        transport: T,
        tool_timeout: u64,
    ) -> Result<Self, McpError>
    where
        T: rust_mcp_sdk::TransportDispatcher<
            rust_mcp_sdk::schema::schema_utils::ServerMessages,
            rust_mcp_sdk::schema::schema_utils::MessageFromClient,
            rust_mcp_sdk::schema::schema_utils::ServerMessage,
            rust_mcp_sdk::schema::schema_utils::ClientMessages,
            rust_mcp_sdk::schema::schema_utils::ClientMessage,
        >,
    {
        let client_details = InitializeRequestParams {
            capabilities: ClientCapabilities::default(),
            client_info: Implementation {
                name: "agent-diva".into(),
                version: "0.4.10".into(),
                title: Some("Agent Diva MCP Client".into()),
                description: Some("Agent Diva MCP Client using rust-mcp-sdk".into()),
                icons: vec![],
                website_url: None,
            },
            protocol_version: LATEST_PROTOCOL_VERSION.into(),
            meta: None,
        };

        let handler = SimpleClientHandler;

        let client = client_runtime::create_client(McpClientOptions {
            client_details,
            transport,
            handler: handler.to_mcp_client_handler(),
            task_store: None,
            server_task_store: None,
            message_observer: None,
        });

        // Handshake/start can hang on dead SSE URLs or stuck child processes; `list_tools`
        // already has a timeout, but we never reach it if `start` never completes.
        let start_timeout_secs = tool_timeout.clamp(10, 120);
        let start_timeout = Duration::from_secs(start_timeout_secs);
        tokio::time::timeout(start_timeout, {
            let client = client.clone();
            async move { client.start().await }
        })
        .await
        .map_err(|_| McpError::Timeout)?
        .map_err(|e| McpError::Sdk(e.to_string()))?;

        Ok(Self {
            server_name: server_name.to_string(),
            client,
            tool_timeout,
        })
    }

    /// List available tools from the server.
    pub async fn list_tools(&self) -> Result<Vec<DiscoveredTool>, McpError> {
        let timeout_duration = Duration::from_secs(self.tool_timeout);

        let result = tokio::time::timeout(timeout_duration, self.client.request_tool_list(None))
            .await
            .map_err(|_| McpError::Timeout)?
            .map_err(|e| McpError::Sdk(e.to_string()))?;

        Ok(result
            .tools
            .into_iter()
            .map(|tool| {
                let mut input_schema = serde_json::to_value(&tool.input_schema)
                    .unwrap_or_else(|_| serde_json::json!({"type": "object", "properties": {}}));

                // Sanitize strings in the schema to remove control characters
                sanitize_json_strings(&mut input_schema);

                // Sanitize description
                let description = sanitize_for_json(&tool.description.unwrap_or_default());

                DiscoveredTool {
                    original_name: tool.name,
                    description,
                    input_schema,
                }
            })
            .collect())
    }

    /// Call a tool on the server.
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<String, McpError> {
        let timeout_duration = Duration::from_secs(self.tool_timeout);

        let params = CallToolRequestParams {
            name: tool_name.into(),
            arguments: Some(arguments.as_object().cloned().unwrap_or_default()),
            meta: None,
            task: None,
        };

        let result = tokio::time::timeout(timeout_duration, self.client.request_tool_call(params))
            .await
            .map_err(|_| McpError::Timeout)?
            .map_err(|e| McpError::Sdk(e.to_string()))?;

        Ok(render_tool_result(&result))
    }

    /// Shutdown the client.
    pub async fn shutdown(&self) {
        let _ = self.client.shut_down().await;
    }

    /// Get the server name.
    pub fn server_name(&self) -> &str {
        &self.server_name
    }
}

/// Simple client handler that handles MCP messages.
struct SimpleClientHandler;

#[async_trait]
impl ClientHandler for SimpleClientHandler {
    /// Handle stderr output from MCP server process.
    ///
    /// Many MCP servers output startup/status messages to stderr (e.g.,
    /// "Context7 Documentation MCP Server v2.1.4 running on stdio").
    /// This is normal behavior, so we log at debug level instead of error.
    async fn handle_process_error(
        &self,
        error_message: String,
        _runtime: &dyn rust_mcp_sdk::McpClient,
    ) -> std::result::Result<(), rust_mcp_sdk::schema::RpcError> {
        // Log at debug level since stderr often contains normal status messages,
        // not actual errors. Many MCP servers use stderr for startup banners.
        tracing::debug!("MCP server stderr: {}", error_message);
        Ok(())
    }
}

fn render_tool_result(result: &rust_mcp_sdk::schema::CallToolResult) -> String {
    let mut parts = Vec::new();

    for content in &result.content {
        if let Ok(text) = content.as_text_content() {
            // Sanitize tool output to remove control characters
            let sanitized = sanitize_for_json(&text.text);
            parts.push(sanitized);
        } else if let Ok(resource) = content.as_resource_link() {
            parts.push(format!("[Resource: {:?}]", resource));
        } else {
            parts.push(format!("[Content: {:?}]", content));
        }
    }

    if parts.is_empty() {
        "(no output)".to_string()
    } else {
        parts.join("\n")
    }
}

// ============================================================================
// MCP Tool Implementation
// ============================================================================

/// MCP tool that wraps a tool from an MCP server.
pub struct McpSdkTool {
    server_name: String,
    client: Arc<RwLock<Option<McpClientWrapper>>>,
    original_name: String,
    wrapped_name: String,
    description: String,
    parameters: Value,
    #[allow(dead_code)]
    tool_timeout: u64,
}

impl McpSdkTool {
    pub fn new(
        server_name: &str,
        client: Arc<RwLock<Option<McpClientWrapper>>>,
        tool: DiscoveredTool,
        tool_timeout: u64,
    ) -> Self {
        let wrapped_name = format!(
            "mcp_{}_{}",
            sanitize_identifier(server_name),
            sanitize_identifier(&tool.original_name)
        );

        Self {
            server_name: server_name.to_string(),
            client,
            original_name: tool.original_name,
            wrapped_name,
            description: format!("[MCP:{}] {}", server_name, tool.description),
            parameters: tool.input_schema,
            tool_timeout,
        }
    }
}

#[async_trait]
impl Tool for McpSdkTool {
    fn name(&self) -> &str {
        &self.wrapped_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> Value {
        self.parameters.clone()
    }

    async fn execute(&self, args: Value) -> agent_diva_tooling::Result<String> {
        if !args.is_object() {
            return Err(ToolError::InvalidArguments(
                "MCP tool arguments must be a JSON object".to_string(),
            ));
        }

        let mut guard = self.client.write().await;

        let client = guard.as_mut().ok_or_else(|| {
            ToolError::ExecutionFailed(format!(
                "MCP server '{}' session is closed",
                self.server_name
            ))
        })?;

        client
            .call_tool(&self.original_name, args)
            .await
            .map_err(|e| {
                ToolError::ExecutionFailed(format!("MCP server '{}': {}", self.server_name, e))
            })
    }
}

fn sanitize_identifier(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "tool".to_string()
    } else {
        out
    }
}

// ============================================================================
// Public API Functions
// ============================================================================

/// Probe an MCP server to discover its tools (one-shot).
pub async fn probe_mcp_server(
    server_name: &str,
    config: &MCPServerConfig,
) -> Result<Vec<DiscoveredTool>, McpError> {
    let client = if !config.command.trim().is_empty() {
        McpClientWrapper::new_stdio(server_name, config).await?
    } else if !config.url.trim().is_empty() {
        McpClientWrapper::new_sse(server_name, config).await?
    } else {
        return Err(McpError::Config(
            "MCP server requires either command or url".to_string(),
        ));
    };

    let tools = client.list_tools().await;
    client.shutdown().await;
    tools
}

/// Load MCP tools from configured servers.
pub async fn load_mcp_tools(
    configs: &HashMap<String, MCPServerConfig>,
) -> HashMap<String, (Arc<RwLock<Option<McpClientWrapper>>>, Vec<DiscoveredTool>)> {
    let mut result = HashMap::new();

    for (server_name, config) in configs {
        match create_client_and_discover_tools(server_name, config).await {
            Ok((client, tools)) => {
                result.insert(server_name.clone(), (client, tools));
            }
            Err(err) => {
                warn!("MCP server '{}' skipped: {}", server_name, err);
            }
        }
    }

    result
}

async fn create_client_and_discover_tools(
    server_name: &str,
    config: &MCPServerConfig,
) -> Result<(Arc<RwLock<Option<McpClientWrapper>>>, Vec<DiscoveredTool>), McpError> {
    let client = if !config.command.trim().is_empty() {
        McpClientWrapper::new_stdio(server_name, config).await?
    } else if !config.url.trim().is_empty() {
        McpClientWrapper::new_sse(server_name, config).await?
    } else {
        return Err(McpError::Config(
            "MCP server requires either command or url".to_string(),
        ));
    };

    let tools = client.list_tools().await?;
    let client_arc = Arc::new(RwLock::new(Some(client)));

    Ok((client_arc, tools))
}

/// Synchronous wrapper for MCP probing.
///
/// This function can be called from both async and non-async contexts.
/// When called from within a tokio runtime, it uses `block_in_place` to
/// avoid the "Cannot start a runtime from within a runtime" error.
pub fn probe_mcp_server_sync(
    server_name: &str,
    config: &MCPServerConfig,
) -> std::result::Result<usize, String> {
    // Try to get the current runtime handle
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // We're inside a runtime, use block_in_place
            tokio::task::block_in_place(|| {
                handle.block_on(async {
                    let tools = probe_mcp_server(server_name, config)
                        .await
                        .map_err(|e| e.to_string())?;
                    Ok(tools.len())
                })
            })
        }
        Err(_) => {
            // Not in a runtime, create one
            let rt = tokio::runtime::Runtime::new()
                .map_err(|e| format!("failed to create tokio runtime: {}", e))?;
            rt.block_on(async {
                let tools = probe_mcp_server(server_name, config)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(tools.len())
            })
        }
    }
}

/// Synchronous wrapper for loading MCP tools as Tool trait objects.
///
/// This function provides backward compatibility with the legacy API,
/// returning `Vec<Arc<dyn Tool>>` for use in non-async contexts.
/// It can be called from both async and non-async contexts.
pub fn load_mcp_tools_sync(configs: &HashMap<String, MCPServerConfig>) -> Vec<Arc<dyn Tool>> {
    let run = || async {
        let loaded = load_mcp_tools(configs).await;
        let mut tools: Vec<Arc<dyn Tool>> = Vec::new();

        for (server_name, (client, discovered_tools)) in loaded {
            let tool_timeout = configs
                .get(&server_name)
                .map(|c| c.tool_timeout)
                .unwrap_or(30);

            for tool in discovered_tools {
                let mcp_tool = McpSdkTool::new(&server_name, client.clone(), tool, tool_timeout);
                tools.push(Arc::new(mcp_tool));
            }
        }

        tools
    };

    // Try to get the current runtime handle
    match tokio::runtime::Handle::try_current() {
        Ok(handle) => {
            // We're inside a runtime, use block_in_place
            tokio::task::block_in_place(|| handle.block_on(run()))
        }
        Err(_) => {
            // Not in a runtime, create one
            let rt = match tokio::runtime::Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    warn!("Failed to create tokio runtime for MCP tools: {}", e);
                    return Vec::new();
                }
            };
            rt.block_on(run())
        }
    }
}
