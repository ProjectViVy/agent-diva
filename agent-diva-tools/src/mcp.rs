//! MCP tools loaded from configured MCP servers.

use super::base::{Result, Tool, ToolError};
use async_trait::async_trait;
use agent_diva_core::config::MCPServerConfig;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Error, ErrorKind, Read, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::warn;

const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Load MCP tools from configured servers.
pub fn load_mcp_tools(configs: &HashMap<String, MCPServerConfig>) -> Vec<Arc<dyn Tool>> {
    let mut tools: Vec<Arc<dyn Tool>> = Vec::new();

    for (server_name, cfg) in configs {
        match discover_server_tools(server_name, cfg) {
            Ok((transport, discovered)) => {
                for tool in discovered {
                    tools.push(Arc::new(McpTool::new(server_name, transport.clone(), tool)));
                }
            }
            Err(err) => {
                warn!("MCP server '{}' skipped: {}", server_name, err);
            }
        }
    }

    tools
}

#[derive(Debug, Clone)]
struct DiscoveredTool {
    original_name: String,
    description: String,
    input_schema: Value,
}

#[derive(Clone)]
enum McpTransport {
    Stdio(MCPServerConfig),
    Http(Arc<Mutex<HttpMcpClient>>),
}

struct McpTool {
    server_name: String,
    transport: McpTransport,
    original_name: String,
    wrapped_name: String,
    description: String,
    parameters: Value,
}

impl McpTool {
    fn new(server_name: &str, transport: McpTransport, tool: DiscoveredTool) -> Self {
        let wrapped_name = format!(
            "mcp_{}_{}",
            sanitize_identifier(server_name),
            sanitize_identifier(&tool.original_name)
        );

        Self {
            server_name: server_name.to_string(),
            transport,
            original_name: tool.original_name,
            wrapped_name,
            description: format!("[MCP:{}] {}", server_name, tool.description),
            parameters: tool.input_schema,
        }
    }
}

#[async_trait]
impl Tool for McpTool {
    fn name(&self) -> &str {
        &self.wrapped_name
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn parameters(&self) -> Value {
        self.parameters.clone()
    }

    async fn execute(&self, args: Value) -> Result<String> {
        if !args.is_object() {
            return Err(ToolError::InvalidArguments(
                "MCP tool arguments must be a JSON object".to_string(),
            ));
        }

        match &self.transport {
            McpTransport::Stdio(cfg) => {
                let cfg = cfg.clone();
                let tool_name = self.original_name.clone();
                let args_obj = args;
                let result = tokio::task::spawn_blocking(move || {
                    invoke_stdio_tool(&cfg, &tool_name, args_obj)
                })
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("MCP execution join error: {}", e))
                })?;
                result.map_err(ToolError::ExecutionFailed)
            }
            McpTransport::Http(client) => {
                let client = Arc::clone(client);
                let tool_name = self.original_name.clone();
                let args_obj = args;
                let result = tokio::task::spawn_blocking(move || {
                    let mut guard = client
                        .lock()
                        .map_err(|_| "MCP HTTP client lock poisoned".to_string())?;
                    guard.call_tool(&tool_name, args_obj)
                })
                .await
                .map_err(|e| {
                    ToolError::ExecutionFailed(format!("MCP execution join error: {}", e))
                })?;
                result.map_err(|e| {
                    ToolError::ExecutionFailed(format!("MCP server '{}': {}", self.server_name, e))
                })
            }
        }
    }
}

fn discover_server_tools(
    server_name: &str,
    cfg: &MCPServerConfig,
) -> std::result::Result<(McpTransport, Vec<DiscoveredTool>), String> {
    if !cfg.command.trim().is_empty() {
        let discovered = discover_stdio_tools(server_name, cfg)?;
        return Ok((McpTransport::Stdio(cfg.clone()), discovered));
    }

    if cfg.url.trim().is_empty() {
        return Err("missing command/url configuration".to_string());
    }

    let mut client = HttpMcpClient::new(cfg.url.clone())?;
    let discovered = client
        .list_tools()
        .map_err(|e| format!("MCP server '{}': {}", server_name, e))?;
    Ok((McpTransport::Http(Arc::new(Mutex::new(client))), discovered))
}

fn discover_stdio_tools(
    server_name: &str,
    cfg: &MCPServerConfig,
) -> std::result::Result<Vec<DiscoveredTool>, String> {
    let mut client = StdioMcpClient::start(cfg)
        .map_err(|e| format!("failed to start stdio MCP process: {}", e))?;

    let tools = (|| -> std::result::Result<Vec<DiscoveredTool>, String> {
        client.initialize()?;
        client.notify_initialized()?;

        let response = client.request("tools/list", json!({}), 2)?;
        extract_tools_from_list_response(&response)
    })();

    client.shutdown();
    tools.map_err(|e| format!("MCP server '{}': {}", server_name, e))
}

fn invoke_stdio_tool(
    cfg: &MCPServerConfig,
    tool_name: &str,
    arguments: Value,
) -> std::result::Result<String, String> {
    let mut client =
        StdioMcpClient::start(cfg).map_err(|e| format!("failed to start MCP process: {}", e))?;

    let result = (|| -> std::result::Result<String, String> {
        client.initialize()?;
        client.notify_initialized()?;
        let response = client.request(
            "tools/call",
            json!({
                "name": tool_name,
                "arguments": arguments
            }),
            3,
        )?;
        Ok(render_tool_result(&response))
    })();

    client.shutdown();
    result
}

fn extract_tools_from_list_response(
    response: &Value,
) -> std::result::Result<Vec<DiscoveredTool>, String> {
    let tools = response
        .get("tools")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "tools/list response missing 'tools' array".to_string())?;

    let mut out = Vec::new();
    for item in tools {
        let original_name = item
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "tool missing name".to_string())?
            .to_string();
        let description = item
            .get("description")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or(&original_name)
            .to_string();
        let input_schema = item
            .get("inputSchema")
            .or_else(|| item.get("input_schema"))
            .cloned()
            .unwrap_or_else(|| json!({"type":"object","properties":{}}));

        out.push(DiscoveredTool {
            original_name,
            description,
            input_schema,
        });
    }
    Ok(out)
}

fn render_tool_result(result: &Value) -> String {
    let mut parts = Vec::new();
    if let Some(items) = result.get("content").and_then(|v| v.as_array()) {
        for item in items {
            if item.get("type").and_then(|v| v.as_str()) == Some("text") {
                if let Some(text) = item.get("text").and_then(|v| v.as_str()) {
                    parts.push(text.to_string());
                    continue;
                }
            }
            parts.push(item.to_string());
        }
    }

    if parts.is_empty() {
        "(no output)".to_string()
    } else {
        parts.join("\n")
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

struct StdioMcpClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl StdioMcpClient {
    fn start(cfg: &MCPServerConfig) -> std::io::Result<Self> {
        let mut cmd = Command::new(cfg.command.trim());
        cmd.args(&cfg.args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        if !cfg.env.is_empty() {
            cmd.envs(&cfg.env);
        }
        let mut child = cmd.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| Error::new(ErrorKind::Other, "missing child stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::new(ErrorKind::Other, "missing child stdout"))?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    fn initialize(&mut self) -> std::result::Result<(), String> {
        let _ = self.request(
            "initialize",
            json!({
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": {
                    "name": "agent-diva",
                    "version": "0.2.0"
                }
            }),
            1,
        )?;
        Ok(())
    }

    fn notify_initialized(&mut self) -> std::result::Result<(), String> {
        self.notify("notifications/initialized", json!({}))
    }

    fn request(
        &mut self,
        method: &str,
        params: Value,
        request_id: u64,
    ) -> std::result::Result<Value, String> {
        self.send_message(&json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params
        }))
        .map_err(|e| format!("failed to write MCP request '{}': {}", method, e))?;

        loop {
            let msg = self
                .read_message()
                .map_err(|e| format!("failed to read MCP response '{}': {}", method, e))?;
            let id = msg.get("id").and_then(|v| v.as_u64());
            if id != Some(request_id) {
                continue;
            }
            if let Some(err) = msg.get("error") {
                return Err(format!("MCP request '{}' failed: {}", method, err));
            }
            return Ok(msg.get("result").cloned().unwrap_or_else(|| json!({})));
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> std::result::Result<(), String> {
        self.send_message(&json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }))
        .map_err(|e| format!("failed to write MCP notification '{}': {}", method, e))
    }

    fn send_message(&mut self, msg: &Value) -> std::io::Result<()> {
        let payload = serde_json::to_vec(msg).map_err(|e| Error::new(ErrorKind::Other, e))?;
        let header = format!("Content-Length: {}\r\n\r\n", payload.len());
        self.stdin.write_all(header.as_bytes())?;
        self.stdin.write_all(&payload)?;
        self.stdin.flush()
    }

    fn read_message(&mut self) -> std::io::Result<Value> {
        let mut content_length = None;
        loop {
            let mut line = String::new();
            let bytes = self.stdout.read_line(&mut line)?;
            if bytes == 0 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "MCP process closed stdout",
                ));
            }

            let trimmed = line.trim_end_matches(['\r', '\n']);
            if trimmed.is_empty() {
                break;
            }

            let lower = trimmed.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("content-length:") {
                let value = rest
                    .trim()
                    .parse::<usize>()
                    .map_err(|e| Error::new(ErrorKind::Other, e))?;
                content_length = Some(value);
            }
        }

        let len = content_length
            .ok_or_else(|| Error::new(ErrorKind::Other, "missing Content-Length header"))?;
        let mut payload = vec![0_u8; len];
        self.stdout.read_exact(&mut payload)?;
        serde_json::from_slice::<Value>(&payload).map_err(|e| Error::new(ErrorKind::Other, e))
    }

    fn shutdown(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

struct HttpMcpClient {
    client: reqwest::blocking::Client,
    url: String,
    session_id: Option<String>,
    protocol_version: String,
    next_id: u64,
    initialized: bool,
}

impl HttpMcpClient {
    fn new(url: String) -> std::result::Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(90))
            .build()
            .map_err(|e| format!("failed to build HTTP client: {}", e))?;

        Ok(Self {
            client,
            url,
            session_id: None,
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            next_id: 1,
            initialized: false,
        })
    }

    fn list_tools(&mut self) -> std::result::Result<Vec<DiscoveredTool>, String> {
        self.ensure_initialized()?;
        let result = self.request("tools/list", json!({}))?;
        extract_tools_from_list_response(&result)
    }

    fn call_tool(
        &mut self,
        tool_name: &str,
        arguments: Value,
    ) -> std::result::Result<String, String> {
        self.ensure_initialized()?;
        let result = self.request(
            "tools/call",
            json!({
                "name": tool_name,
                "arguments": arguments
            }),
        )?;
        Ok(render_tool_result(&result))
    }

    fn ensure_initialized(&mut self) -> std::result::Result<(), String> {
        if self.initialized {
            return Ok(());
        }

        let init_result = self.request_raw(
            "initialize",
            json!({
                "protocolVersion": self.protocol_version,
                "capabilities": {},
                "clientInfo": {
                    "name": "agent-diva",
                    "version": "0.2.0"
                }
            }),
            false,
        )?;

        if let Some(version) = init_result
            .get("protocolVersion")
            .and_then(|v| v.as_str())
            .filter(|v| !v.trim().is_empty())
        {
            self.protocol_version = version.to_string();
        }

        self.notify("notifications/initialized", json!({}))?;
        self.initialized = true;
        Ok(())
    }

    fn request(&mut self, method: &str, params: Value) -> std::result::Result<Value, String> {
        self.request_raw(method, params, true)
    }

    fn request_raw(
        &mut self,
        method: &str,
        params: Value,
        allow_reinitialize: bool,
    ) -> std::result::Result<Value, String> {
        let request_id = self.take_request_id();
        let payload = json!({
            "jsonrpc": "2.0",
            "id": request_id,
            "method": method,
            "params": params
        });

        match self.post_and_wait_for_response(&payload, request_id) {
            Ok(v) => Ok(v),
            Err(e) => {
                if allow_reinitialize
                    && self.session_id.is_some()
                    && e.contains("404")
                    && method != "initialize"
                {
                    self.initialized = false;
                    self.session_id = None;
                    self.ensure_initialized()?;
                    self.request_raw(
                        method,
                        payload.get("params").cloned().unwrap_or_else(|| json!({})),
                        false,
                    )
                } else {
                    Err(e)
                }
            }
        }
    }

    fn notify(&mut self, method: &str, params: Value) -> std::result::Result<(), String> {
        let payload = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let mut req = self
            .client
            .post(&self.url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                reqwest::header::ACCEPT,
                "application/json, text/event-stream",
            )
            .header("MCP-Protocol-Version", self.protocol_version.clone())
            .json(&payload);

        if let Some(session_id) = &self.session_id {
            req = req.header("MCP-Session-Id", session_id);
        }

        let response = req
            .send()
            .map_err(|e| format!("HTTP MCP notification failed: {}", e))?;
        self.capture_session_header(response.headers());

        if response.status().is_success() {
            Ok(())
        } else {
            Err(format!(
                "HTTP MCP notification returned {}",
                response.status()
            ))
        }
    }

    fn post_and_wait_for_response(
        &mut self,
        payload: &Value,
        request_id: u64,
    ) -> std::result::Result<Value, String> {
        let mut req = self
            .client
            .post(&self.url)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .header(
                reqwest::header::ACCEPT,
                "application/json, text/event-stream",
            )
            .header("MCP-Protocol-Version", self.protocol_version.clone())
            .json(payload);

        if let Some(session_id) = &self.session_id {
            req = req.header("MCP-Session-Id", session_id);
        }

        let response = req
            .send()
            .map_err(|e| format!("HTTP MCP request failed: {}", e))?;

        self.capture_session_header(response.headers());

        if response.status().as_u16() == 404 {
            return Err("HTTP MCP request failed with 404".to_string());
        }
        if !response.status().is_success() {
            return Err(format!("HTTP MCP request returned {}", response.status()));
        }

        let content_type = response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_ascii_lowercase();

        if content_type.contains("application/json") {
            let msg: Value = response
                .json()
                .map_err(|e| format!("failed to decode MCP JSON response: {}", e))?;
            return extract_rpc_result(msg, request_id);
        }

        if content_type.contains("text/event-stream") {
            let (result, last_event_id, retry_ms) = consume_sse_stream(response, request_id)?;
            if let Some(value) = result {
                return Ok(value);
            }

            let mut last_event_id = last_event_id;
            let mut retry_ms = retry_ms.unwrap_or(1000);

            loop {
                let event_id = last_event_id
                    .as_ref()
                    .ok_or_else(|| {
                        "MCP SSE stream ended before response and no event id available".to_string()
                    })?
                    .clone();

                std::thread::sleep(Duration::from_millis(retry_ms));

                let mut poll_req = self
                    .client
                    .get(&self.url)
                    .header(reqwest::header::ACCEPT, "text/event-stream")
                    .header("MCP-Protocol-Version", self.protocol_version.clone())
                    .header("Last-Event-ID", event_id);
                if let Some(session_id) = &self.session_id {
                    poll_req = poll_req.header("MCP-Session-Id", session_id);
                }

                let poll_response = poll_req
                    .send()
                    .map_err(|e| format!("MCP SSE reconnect failed: {}", e))?;
                self.capture_session_header(poll_response.headers());

                if !poll_response.status().is_success() {
                    return Err(format!(
                        "MCP SSE reconnect returned {}",
                        poll_response.status()
                    ));
                }

                let (poll_result, poll_last_event_id, poll_retry_ms) =
                    consume_sse_stream(poll_response, request_id)?;
                if let Some(value) = poll_result {
                    return Ok(value);
                }
                if let Some(id) = poll_last_event_id {
                    last_event_id = Some(id);
                }
                if let Some(ms) = poll_retry_ms {
                    retry_ms = ms;
                }
            }
        }

        Err(format!(
            "unsupported MCP HTTP response content-type: {}",
            content_type
        ))
    }

    fn capture_session_header(&mut self, headers: &reqwest::header::HeaderMap) {
        if let Some(value) = headers.get("MCP-Session-Id").and_then(|v| v.to_str().ok()) {
            if !value.trim().is_empty() {
                self.session_id = Some(value.to_string());
            }
        }
    }

    fn take_request_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }
}

#[derive(Default)]
struct SseEvent {
    data_lines: Vec<String>,
    id: Option<String>,
    retry_ms: Option<u64>,
}

impl SseEvent {
    fn data(&self) -> String {
        self.data_lines.join("\n")
    }

    fn is_empty(&self) -> bool {
        self.data_lines.is_empty() && self.id.is_none() && self.retry_ms.is_none()
    }
}

fn consume_sse_stream(
    response: reqwest::blocking::Response,
    request_id: u64,
) -> std::result::Result<(Option<Value>, Option<String>, Option<u64>), String> {
    let mut reader = BufReader::new(response);
    let mut last_event_id: Option<String> = None;
    let mut retry_ms: Option<u64> = None;

    while let Some(event) = read_sse_event(&mut reader)? {
        if let Some(id) = &event.id {
            last_event_id = Some(id.clone());
        }
        if let Some(ms) = event.retry_ms {
            retry_ms = Some(ms);
        }

        let data = event.data();
        if data.trim().is_empty() {
            continue;
        }

        let msg: Value = match serde_json::from_str(&data) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if let Some(id) = msg.get("id").and_then(|v| v.as_u64()) {
            if id == request_id {
                let value = extract_rpc_result(msg, request_id)?;
                return Ok((Some(value), last_event_id, retry_ms));
            }
        }
    }

    Ok((None, last_event_id, retry_ms))
}

fn read_sse_event<R: BufRead>(reader: &mut R) -> std::result::Result<Option<SseEvent>, String> {
    let mut event = SseEvent::default();

    loop {
        let mut line = String::new();
        let bytes = reader
            .read_line(&mut line)
            .map_err(|e| format!("failed to read SSE stream: {}", e))?;

        if bytes == 0 {
            if event.is_empty() {
                return Ok(None);
            }
            return Ok(Some(event));
        }

        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            if event.is_empty() {
                continue;
            }
            return Ok(Some(event));
        }

        if let Some(rest) = line.strip_prefix(':') {
            let _ = rest;
            continue;
        }

        if let Some(rest) = line.strip_prefix("data:") {
            event.data_lines.push(rest.trim_start().to_string());
        } else if let Some(rest) = line.strip_prefix("id:") {
            event.id = Some(rest.trim_start().to_string());
        } else if let Some(rest) = line.strip_prefix("retry:") {
            if let Ok(ms) = rest.trim_start().parse::<u64>() {
                event.retry_ms = Some(ms);
            }
        }
    }
}

fn extract_rpc_result(msg: Value, request_id: u64) -> std::result::Result<Value, String> {
    let id = msg.get("id").and_then(|v| v.as_u64());
    if id != Some(request_id) {
        return Err(format!(
            "received mismatched MCP response id: expected {}, got {:?}",
            request_id, id
        ));
    }

    if let Some(err) = msg.get("error") {
        return Err(format!("MCP request failed: {}", err));
    }

    Ok(msg.get("result").cloned().unwrap_or_else(|| json!({})))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_sse_event_parses_data_and_id() {
        let input = "id: 11\ndata: {\"jsonrpc\":\"2.0\",\"id\":1,\"result\":{}}\n\n";
        let mut reader = BufReader::new(input.as_bytes());
        let event = read_sse_event(&mut reader).unwrap().unwrap();
        assert_eq!(event.id.as_deref(), Some("11"));
        assert!(event.data().contains("\"jsonrpc\""));
    }

    #[test]
    fn test_extract_rpc_result_ok() {
        let msg = json!({
            "jsonrpc": "2.0",
            "id": 7,
            "result": {"ok": true}
        });

        let result = extract_rpc_result(msg, 7).unwrap();
        assert_eq!(result["ok"], json!(true));
    }
}
