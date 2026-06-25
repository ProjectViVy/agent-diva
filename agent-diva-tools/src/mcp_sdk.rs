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

/// Maximum characters allowed in a tool result before truncation.
/// Prevents oversized MCP responses from blowing up the LLM context window.
const MAX_TOOL_RESULT_CHARS: usize = 100_000;

/// Maximum number of retry attempts for transient MCP failures.
const MAX_RETRIES: u32 = 3;

/// Base delay for exponential backoff in milliseconds.
const BACKOFF_BASE_MS: u64 = 500;

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

type SharedMcpClient = Arc<RwLock<Option<Arc<McpClientWrapper>>>>;

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

    /// Call a tool on the server with exponential backoff retry for transient failures.
    pub async fn call_tool(&self, tool_name: &str, arguments: Value) -> Result<String, McpError> {
        let timeout_duration = Duration::from_secs(self.tool_timeout);

        let params = CallToolRequestParams {
            name: tool_name.into(),
            arguments: Some(arguments.as_object().cloned().unwrap_or_default()),
            meta: None,
            task: None,
        };

        let mut last_err: Option<McpError> = None;

        for attempt in 0..=MAX_RETRIES {
            if attempt > 0 {
                let delay_ms = BACKOFF_BASE_MS * 2u64.pow(attempt - 1);
                warn!(
                    "MCP tool '{}' retry {}/{} after {}ms backoff",
                    tool_name, attempt, MAX_RETRIES, delay_ms
                );
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
            }

            let result = tokio::time::timeout(
                timeout_duration,
                self.client.request_tool_call(params.clone()),
            )
            .await
            .map_err(|_| McpError::Timeout)?
            .map_err(|e| McpError::Sdk(e.to_string()));

            match result {
                Ok(call_result) => return render_tool_result(&call_result),
                Err(e) => {
                    // Only retry on transient errors (timeout, connection)
                    let is_transient = matches!(
                        e,
                        McpError::Timeout | McpError::ConnectionFailed(_)
                    );
                    last_err = Some(e);
                    if !is_transient {
                        break;
                    }
                }
            }
        }

        Err(last_err.unwrap_or(McpError::Sdk("unknown retry failure".into())))
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

fn render_tool_result(result: &rust_mcp_sdk::schema::CallToolResult) -> Result<String, McpError> {
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

    let rendered = if parts.is_empty() {
        "(no output)".to_string()
    } else {
        parts.join("\n")
    };

    // C-5: Truncate oversized results to protect LLM context window
    let rendered = if rendered.len() > MAX_TOOL_RESULT_CHARS {
        let truncated: String = rendered.chars().take(MAX_TOOL_RESULT_CHARS).collect();
        format!(
            "{}\n\n[truncated: output exceeded {} chars]",
            truncated, MAX_TOOL_RESULT_CHARS
        )
    } else {
        rendered
    };

    if matches!(result.is_error, Some(true)) {
        Err(McpError::Server(rendered))
    } else {
        Ok(rendered)
    }
}

// ============================================================================
// MCP Tool Implementation
// ============================================================================

/// MCP tool that wraps a tool from an MCP server.
pub struct McpSdkTool {
    server_name: String,
    client: SharedMcpClient,
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
        client: SharedMcpClient,
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

        let client = clone_live_client(&self.client, &self.server_name).await?;

        client
            .call_tool(&self.original_name, args)
            .await
            .map_err(|e| map_mcp_error_to_tool_error(&self.server_name, e))
    }
}

fn map_mcp_error_to_tool_error(server_name: &str, error: McpError) -> ToolError {
    match error {
        McpError::Server(message) => {
            ToolError::Error(format!("MCP server '{}': {}", server_name, message))
        }
        other => ToolError::ExecutionFailed(format!("MCP server '{}': {}", server_name, other)),
    }
}

async fn clone_live_client<T>(
    client: &Arc<RwLock<Option<Arc<T>>>>,
    server_name: &str,
) -> agent_diva_tooling::Result<Arc<T>>
where
    T: Send + Sync + 'static,
{
    let guard = client.read().await;
    guard.as_ref().cloned().ok_or_else(|| {
        ToolError::ExecutionFailed(format!("MCP server '{}' session is closed", server_name))
    })
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
) -> HashMap<String, (SharedMcpClient, Vec<DiscoveredTool>)> {
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
) -> Result<(SharedMcpClient, Vec<DiscoveredTool>), McpError> {
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
    let client_arc = Arc::new(RwLock::new(Some(Arc::new(client))));

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

#[cfg(test)]
mod tests {
    use super::{
        clone_live_client, map_mcp_error_to_tool_error, render_tool_result, McpError,
        MAX_TOOL_RESULT_CHARS,
    };
    use agent_diva_tooling::ToolError;
    use rust_mcp_sdk::schema::{CallToolResult, ContentBlock, TextContent};
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };
    use std::time::Duration;
    use tokio::sync::{Barrier, RwLock};

    #[test]
    fn render_tool_result_returns_server_error_when_is_error_is_true() {
        let result = CallToolResult {
            content: vec![ContentBlock::TextContent(TextContent::new(
                "tool failed".into(),
                None,
                None,
            ))],
            is_error: Some(true),
            meta: None,
            structured_content: None,
        };

        let error = render_tool_result(&result).expect_err("expected MCP tool failure");

        assert!(matches!(error, McpError::Server(message) if message == "tool failed"));
    }

    #[test]
    fn render_tool_result_returns_success_output_when_is_error_is_missing() {
        let result = CallToolResult {
            content: vec![ContentBlock::TextContent(TextContent::new(
                "tool succeeded".into(),
                None,
                None,
            ))],
            is_error: None,
            meta: None,
            structured_content: None,
        };

        let output = render_tool_result(&result).expect("expected MCP tool success");

        assert_eq!(output, "tool succeeded");
    }

    #[test]
    fn map_mcp_error_to_tool_error_uses_tool_error_for_server_failures() {
        let error =
            map_mcp_error_to_tool_error("demo", McpError::Server("tool failed".to_string()));

        assert!(matches!(
            error,
            ToolError::Error(message) if message == "MCP server 'demo': tool failed"
        ));
    }

    #[test]
    fn map_mcp_error_to_tool_error_keeps_execution_failed_for_transport_failures() {
        let error = map_mcp_error_to_tool_error("demo", McpError::Timeout);

        assert!(matches!(
            error,
            ToolError::ExecutionFailed(message)
                if message == "MCP server 'demo': MCP request timed out"
        ));
    }

    struct FakeParallelClient {
        barrier: Barrier,
        active_calls: AtomicUsize,
        max_active_calls: AtomicUsize,
    }

    impl FakeParallelClient {
        fn new(parties: usize) -> Self {
            Self {
                barrier: Barrier::new(parties),
                active_calls: AtomicUsize::new(0),
                max_active_calls: AtomicUsize::new(0),
            }
        }

        async fn call(&self) {
            let active = self.active_calls.fetch_add(1, Ordering::SeqCst) + 1;
            let _ =
                self.max_active_calls
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |current| {
                        (active > current).then_some(active)
                    });

            self.barrier.wait().await;
            tokio::time::sleep(Duration::from_millis(20)).await;
            self.active_calls.fetch_sub(1, Ordering::SeqCst);
        }

        fn max_active_calls(&self) -> usize {
            self.max_active_calls.load(Ordering::SeqCst)
        }
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn clone_live_client_allows_parallel_calls_on_same_server() {
        let fake_client = Arc::new(FakeParallelClient::new(2));
        let shared_client = Arc::new(RwLock::new(Some(fake_client.clone())));

        tokio::time::timeout(Duration::from_millis(200), async {
            let first = async {
                let client = clone_live_client(&shared_client, "demo")
                    .await
                    .expect("first task should clone the live client");
                client.call().await;
            };
            let second = async {
                let client = clone_live_client(&shared_client, "demo")
                    .await
                    .expect("second task should clone the live client");
                client.call().await;
            };

            tokio::join!(first, second);
        })
        .await
        .expect("parallel MCP calls should not be serialized by the session lock");

        assert!(
            fake_client.max_active_calls() >= 2,
            "expected overlapping execution on the shared MCP client"
        );
    }

    // =========================================================================
    // C-5: Result size limit tests
    // =========================================================================

    #[test]
    fn render_tool_result_truncates_output_exceeding_max_chars() {
        let oversized = "x".repeat(MAX_TOOL_RESULT_CHARS + 1000);
        let result = CallToolResult {
            content: vec![ContentBlock::TextContent(TextContent::new(
                oversized.clone(),
                None,
                None,
            ))],
            is_error: None,
            meta: None,
            structured_content: None,
        };

        let output = render_tool_result(&result).expect("expected success");

        assert!(
            output.len() < oversized.len(),
            "output should be shorter than the oversized input"
        );
        assert!(
            output.contains("[truncated:"),
            "output should contain truncation notice"
        );
        assert!(
            output.contains(&MAX_TOOL_RESULT_CHARS.to_string()),
            "output should reference the limit"
        );
    }

    #[test]
    fn render_tool_result_does_not_truncate_output_within_limit() {
        let normal = "hello world".to_string();
        let result = CallToolResult {
            content: vec![ContentBlock::TextContent(TextContent::new(
                normal.clone(),
                None,
                None,
            ))],
            is_error: None,
            meta: None,
            structured_content: None,
        };

        let output = render_tool_result(&result).expect("expected success");

        assert_eq!(output, normal, "normal output should not be truncated");
    }

    #[test]
    fn render_tool_result_truncates_error_output_exceeding_max_chars() {
        let oversized = "e".repeat(MAX_TOOL_RESULT_CHARS + 500);
        let result = CallToolResult {
            content: vec![ContentBlock::TextContent(TextContent::new(
                oversized,
                None,
                None,
            ))],
            is_error: Some(true),
            meta: None,
            structured_content: None,
        };

        let error = render_tool_result(&result).expect_err("expected error");
        match error {
            McpError::Server(msg) => {
                assert!(
                    msg.contains("[truncated:"),
                    "error message should be truncated"
                );
            }
            other => panic!("expected Server error, got: {:?}", other),
        }
    }
}
