//! Background process management tool
//!
//! This tool provides lifecycle management for child processes — spawn, list,
//! poll, log, wait, kill, stdin write, and stdin close. Each process is
//! identified by a caller-supplied `session_id` string and runs asynchronously
//! in the background. Output is captured into in-memory buffers that can be
//! retrieved on demand.

use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// A handle to a managed child process.
///
/// Holds the child process, its PID, captured output buffers, start time,
/// and an optional stdin handle for interactive processes.
struct ProcessHandle {
    /// Child process, protected by a mutex for safe concurrent access.
    child: Mutex<Child>,
    /// Process ID (OS-assigned).
    pid: u32,
    /// Captured stdout bytes, continuously filled by a background reader task.
    stdout_buf: Arc<Mutex<Vec<u8>>>,
    /// Captured stderr bytes, continuously filled by a background reader task.
    stderr_buf: Arc<Mutex<Vec<u8>>>,
    /// Timestamp when the process was spawned.
    started_at: Instant,
    /// Optional stdin handle for writing data to the process.
    stdin: Option<Mutex<ChildStdin>>,
}

/// Background process management tool.
///
/// Allows spawning, monitoring, and controlling child processes asynchronously.
/// Each process is identified by a `session_id` string provided by the caller.
///
/// # Operations
///
/// | Action   | Description                                      |
/// |----------|--------------------------------------------------|
/// | `submit` | Spawn a new child process                        |
/// | `list`   | List all active processes                        |
/// | `poll`   | Check whether a process is still running         |
/// | `log`    | Retrieve accumulated stdout / stderr output      |
/// | `wait`   | Block until the process exits (optional timeout) |
/// | `kill`   | Terminate a process forcefully                   |
/// | `write`  | Write data to the process's stdin                |
/// | `close`  | Close the process's stdin pipe                   |
pub struct ProcessTool {
    /// Map of session_id → process handle.
    processes: Arc<Mutex<HashMap<String, ProcessHandle>>>,
    /// Maximum number of concurrent processes allowed.
    max_concurrent: usize,
    /// Base working directory for spawned processes, if set.
    workspace_dir: Option<std::path::PathBuf>,
}

impl ProcessTool {
    /// Create a new `ProcessTool` with default settings.
    ///
    /// Defaults: `max_concurrent = 5`, no workspace directory override.
    pub fn new() -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            max_concurrent: 5,
            workspace_dir: None,
        }
    }

    /// Set the maximum number of concurrent processes.
    pub fn with_max_concurrent(mut self, max: usize) -> Self {
        self.max_concurrent = max;
        self
    }

    /// Set the workspace directory that spawned processes will run inside.
    pub fn with_workspace_dir(mut self, dir: Option<std::path::PathBuf>) -> Self {
        self.workspace_dir = dir;
        self
    }

    // ── Internal helpers ────────────────────────────────────────────────

    /// Look up a process handle by session_id and return the pid + started_at.
    fn lookup_meta(&self, processes: &HashMap<String, ProcessHandle>, session_id: &str) -> Result<(u32, Instant), ToolError> {
        let handle = processes
            .get(session_id)
            .ok_or_else(|| ToolError::InvalidParams(format!("Unknown session_id: {}", session_id)))?;
        Ok((handle.pid, handle.started_at))
    }

    /// Check whether a child process is still alive via `try_wait`.
    async fn is_running(child: &Mutex<Child>) -> bool {
        match child.lock().await.try_wait() {
            Ok(Some(_)) => false, // exited
            _ => true,            // still running or error → assume running
        }
    }

    /// Spawn background tasks that continuously read stdout / stderr into
    /// the provided atomic buffers.
    fn spawn_pipe_readers(
        child: &mut Child,
        session_id: &str,
        stdout_buf: Arc<Mutex<Vec<u8>>>,
        stderr_buf: Arc<Mutex<Vec<u8>>>,
    ) {
        let sid_out = session_id.to_owned();
        let sid_err = session_id.to_owned();

        if let Some(stdout) = child.stdout.take() {
            let buf = stdout_buf;
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut reader = stdout;
                let mut tmp = [0u8; 8192];
                loop {
                    match reader.read(&mut tmp).await {
                        Ok(0) => break,
                        Ok(n) => {
                            buf.lock().await.extend_from_slice(&tmp[..n]);
                        }
                        Err(e) => {
                            warn!("[process] stdout reader error for {}: {}", sid_out, e);
                            break;
                        }
                    }
                }
                debug!("[process] stdout reader finished for {}", sid_out);
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let buf = stderr_buf;
            tokio::spawn(async move {
                use tokio::io::AsyncReadExt;
                let mut reader = stderr;
                let mut tmp = [0u8; 8192];
                loop {
                    match reader.read(&mut tmp).await {
                        Ok(0) => break,
                        Ok(n) => {
                            buf.lock().await.extend_from_slice(&tmp[..n]);
                        }
                        Err(e) => {
                            warn!("[process] stderr reader error for {}: {}", sid_err, e);
                            break;
                        }
                    }
                }
                debug!("[process] stderr reader finished for {}", sid_err);
            });
        }
    }

    // ── Operation handlers ──────────────────────────────────────────────

    /// `submit` — spawn a new process.
    async fn op_submit(&self, params: &Value) -> Result<String, ToolError> {
        let session_id = params
            .get("session_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'session_id' parameter".into()))?;
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'command' parameter".into()))?;

        // Enforce max_concurrent limit
        {
            let procs = self.processes.lock().await;
            if procs.len() >= self.max_concurrent {
                return Err(ToolError::ExecutionFailed(format!(
                    "Maximum concurrent processes ({}) reached",
                    self.max_concurrent
                )));
            }
        }

        // Determine shell command wrapper per platform
        let (shell, shell_args): (&str, &[&str]) = if cfg!(target_os = "windows") {
            ("powershell", &["-NoProfile", "-NonInteractive", "-Command"])
        } else {
            ("sh", &["-c"])
        };

        let mut cmd = Command::new(shell);
        cmd.args(shell_args);
        cmd.arg(command);
        cmd.stdin(std::process::Stdio::piped());
        cmd.stdout(std::process::Stdio::piped());
        cmd.stderr(std::process::Stdio::piped());

        if let Some(ref dir) = self.workspace_dir {
            cmd.current_dir(dir);
        }

        let mut child = cmd.spawn().map_err(|e| {
            ToolError::ExecutionFailed(format!("Failed to spawn process: {}", e))
        })?;

        let pid = child.id().unwrap_or(0);
        info!(
            "[process] spawned: session_id={}, pid={}, command='{}'",
            session_id, pid, command
        );

        let stdout_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));
        let stderr_buf: Arc<Mutex<Vec<u8>>> = Arc::new(Mutex::new(Vec::new()));

        Self::spawn_pipe_readers(&mut child, session_id, stdout_buf.clone(), stderr_buf.clone());

        let stdin = child.stdin.take().map(Mutex::new);

        let handle = ProcessHandle {
            child: Mutex::new(child),
            pid,
            stdout_buf,
            stderr_buf,
            started_at: Instant::now(),
            stdin,
        };

        {
            let mut procs = self.processes.lock().await;
            procs.insert(session_id.to_owned(), handle);
        }

        Ok(json!({ "pid": pid, "session_id": session_id }).to_string())
    }

    /// `list` — return metadata for all known processes.
    async fn op_list(&self) -> Result<String, ToolError> {
        let procs = self.processes.lock().await;
        let mut list: Vec<Value> = Vec::with_capacity(procs.len());

        for (sid, handle) in procs.iter() {
            let running = Self::is_running(&handle.child).await;
            list.push(json!({
                "session_id": sid,
                "pid": handle.pid,
                "running": running,
                "uptime_secs": handle.started_at.elapsed().as_secs_f64(),
            }));
        }

        Ok(json!({ "processes": list }).to_string())
    }

    /// `poll` — check whether a specific process is still running.
    async fn op_poll(&self, session_id: &str) -> Result<String, ToolError> {
        let procs = self.processes.lock().await;
        let (pid, _) = self.lookup_meta(&procs, session_id)?;
        let handle = procs.get(session_id).unwrap(); // safe: lookup_meta validated
        let running = Self::is_running(&handle.child).await;

        Ok(json!({ "running": running, "pid": pid }).to_string())
    }

    /// `log` — return accumulated stdout / stderr output.
    async fn op_log(&self, session_id: &str, limit: Option<usize>) -> Result<String, ToolError> {
        let procs = self.processes.lock().await;
        let handle = procs
            .get(session_id)
            .ok_or_else(|| ToolError::InvalidParams(format!("Unknown session_id: {}", session_id)))?;

        let stdout_bytes = handle.stdout_buf.lock().await.clone();
        let stderr_bytes = handle.stderr_buf.lock().await.clone();

        let stdout = decode_pipe_bytes(&stdout_bytes);
        let stderr = decode_pipe_bytes(&stderr_bytes);

        let trunc = |s: String, max: usize| -> String {
            if s.len() > max {
                let (head, _) = s.split_at(max);
                format!("{}… (truncated, {} more bytes)", head, s.len() - max)
            } else {
                s
            }
        };

        let limit = limit.unwrap_or(usize::MAX);
        Ok(json!({ "stdout": trunc(stdout, limit), "stderr": trunc(stderr, limit) }).to_string())
    }

    /// `wait` — block until the process exits, with an optional timeout.
    async fn op_wait(&self, session_id: &str, wait_timeout: Option<u64>) -> Result<String, ToolError> {
        // Snapshot pid and started_at while holding the lock briefly.
        let (pid, started_at) = {
            let procs = self.processes.lock().await;
            self.lookup_meta(&procs, session_id)?
        };

        let exit_status = {
            let procs = self.processes.lock().await;
            let handle = procs
                .get(session_id)
                .ok_or_else(|| ToolError::InvalidParams(format!("Unknown session_id: {}", session_id)))?;
            let mut child = handle.child.lock().await;

            match wait_timeout {
                Some(secs) => match timeout(Duration::from_secs(secs), child.wait()).await {
                    Ok(Ok(status)) => status,
                    Ok(Err(e)) => {
                        return Err(ToolError::ExecutionFailed(format!("Wait error: {}", e)));
                    }
                    Err(_) => {
                        // Timeout — process is still running, return without removing
                        return Ok(json!({
                            "timed_out": true,
                            "pid": pid,
                            "uptime_secs": started_at.elapsed().as_secs_f64(),
                        })
                        .to_string());
                    }
                },
                None => child.wait().await.map_err(|e| {
                    ToolError::ExecutionFailed(format!("Wait error: {}", e))
                })?,
            }
        };

        let uptime = started_at.elapsed().as_secs_f64();
        let code = exit_status.code();

        // Remove from map after the process has been reaped.
        self.processes.lock().await.remove(session_id);

        info!("[process] finished: session_id={}, pid={}, exit_code={:?}", session_id, pid, code);

        Ok(json!({
            "exit_code": code,
            "pid": pid,
            "uptime_secs": uptime,
            "timed_out": false,
        })
        .to_string())
    }

    /// `kill` — terminate a process forcefully.
    async fn op_kill(&self, session_id: &str) -> Result<String, ToolError> {
        let (pid, started_at) = {
            let procs = self.processes.lock().await;
            self.lookup_meta(&procs, session_id)?
        };

        let exit_status = {
            let procs = self.processes.lock().await;
            let handle = procs
                .get(session_id)
                .ok_or_else(|| ToolError::InvalidParams(format!("Unknown session_id: {}", session_id)))?;
            let mut child = handle.child.lock().await;

            // Ignore kill errors if the process already exited.
            let _ = child.kill().await;

            child.wait().await.map_err(|e| {
                ToolError::ExecutionFailed(format!("Failed to wait after kill: {}", e))
            })?
        };

        let uptime = started_at.elapsed().as_secs_f64();
        let code = exit_status.code();

        self.processes.lock().await.remove(session_id);

        info!(
            "[process] killed: session_id={}, pid={}, exit_code={:?}",
            session_id, pid, code
        );

        Ok(json!({ "exit_code": code, "pid": pid, "uptime_secs": uptime }).to_string())
    }

    /// `write` — write data to the process's stdin.
    async fn op_write(&self, session_id: &str, data: &str) -> Result<String, ToolError> {
        let procs = self.processes.lock().await;
        let handle = procs
            .get(session_id)
            .ok_or_else(|| ToolError::InvalidParams(format!("Unknown session_id: {}", session_id)))?;

        match &handle.stdin {
            Some(stdin) => {
                let mut writer = stdin.lock().await;
                writer.write_all(data.as_bytes()).await.map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to write to stdin: {}", e))
                })?;
                // Flush to ensure data reaches the child process.
                writer.flush().await.map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to flush stdin: {}", e))
                })?;
                Ok(json!({ "written": data.len() }).to_string())
            }
            None => Err(ToolError::ExecutionFailed(
                "stdin not available for this process".into(),
            )),
        }
    }

    /// `close` — close the process's stdin pipe.
    async fn op_close(&self, session_id: &str) -> Result<String, ToolError> {
        let procs = self.processes.lock().await;
        let handle = procs
            .get(session_id)
            .ok_or_else(|| ToolError::InvalidParams(format!("Unknown session_id: {}", session_id)))?;

        match &handle.stdin {
            Some(stdin) => {
                let mut writer = stdin.lock().await;
                writer.shutdown().await.map_err(|e| {
                    ToolError::ExecutionFailed(format!("Failed to close stdin: {}", e))
                })?;
                Ok(json!({ "stdin_closed": true }).to_string())
            }
            None => Err(ToolError::ExecutionFailed(
                "stdin not available for this process".into(),
            )),
        }
    }
}

impl Default for ProcessTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ProcessTool {
    fn name(&self) -> &str {
        "process"
    }

    fn description(&self) -> &str {
        "后台进程管理工具。可以创建、监控和管理后台进程，\
         支持进程的启动、列表查看、状态检查、日志获取、等待完成、\
         终止、标准输入写入和关闭标准输入等操作。"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["submit", "list", "poll", "log", "wait", "kill", "write", "close"],
                    "description": "操作类型：submit（创建进程）、\
                        list（列出进程）、poll（检查进程状态）、\
                        log（获取进程日志）、wait（等待进程结束）、\
                        kill（终止进程）、write（写入标准输入）、\
                        close（关闭标准输入）"
                },
                "session_id": {
                    "type": "string",
                    "description": "进程会话标识符，用于后续操作引用该进程"
                },
                "command": {
                    "type": "string",
                    "description": "要执行的 Shell 命令（仅 submit 操作需要）"
                },
                "data": {
                    "type": "string",
                    "description": "写入标准输入的数据（仅 write 操作需要）"
                },
                "timeout": {
                    "type": "integer",
                    "description": "等待超时秒数（仅 wait 操作可选使用）"
                },
                "limit": {
                    "type": "integer",
                    "description": "日志输出限制字节数（仅 log 操作可选使用）"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'action' parameter".into()))?;

        debug!("[process] execute action={}", action);

        match action {
            "submit" => self.op_submit(&params).await,
            "list" => self.op_list().await,
            "poll" => {
                let sid = params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidParams("Missing 'session_id' parameter".into()))?;
                self.op_poll(sid).await
            }
            "log" => {
                let sid = params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidParams("Missing 'session_id' parameter".into()))?;
                let limit = params.get("limit").and_then(|v| v.as_u64()).map(|v| v as usize);
                self.op_log(sid, limit).await
            }
            "wait" => {
                let sid = params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidParams("Missing 'session_id' parameter".into()))?;
                let timeout_secs = params.get("timeout").and_then(|v| v.as_u64());
                self.op_wait(sid, timeout_secs).await
            }
            "kill" => {
                let sid = params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidParams("Missing 'session_id' parameter".into()))?;
                self.op_kill(sid).await
            }
            "write" => {
                let sid = params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidParams("Missing 'session_id' parameter".into()))?;
                let data = params
                    .get("data")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidParams("Missing 'data' parameter".into()))?;
                self.op_write(sid, data).await
            }
            "close" => {
                let sid = params
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ToolError::InvalidParams("Missing 'session_id' parameter".into()))?;
                self.op_close(sid).await
            }
            _ => Err(ToolError::InvalidParams(format!(
                "Unknown action '{}'. Valid actions: submit, list, poll, log, wait, kill, write, close",
                action
            ))),
        }
    }
}

// ── Pipe decoding (mirrors shell.rs) ────────────────────────────────────

/// Decode bytes captured from a child process pipe.
///
/// On Windows, PowerShell and cmd often emit system ANSI (e.g. GBK on zh-CN);
/// treating that as UTF-8 produces U+FFFD replacement characters. This function
/// tries strict UTF-8 first, then falls back to GB18030 when it yields fewer
/// replacement characters than lossy UTF-8.
fn decode_pipe_bytes(bytes: &[u8]) -> String {
    if bytes.is_empty() {
        return String::new();
    }
    #[cfg(windows)]
    {
        if std::str::from_utf8(bytes).is_ok() {
            return String::from_utf8_lossy(bytes).into_owned();
        }
        let (gb, _) = encoding_rs::GB18030.decode_without_bom_handling(bytes);
        let lossy_utf8 = String::from_utf8_lossy(bytes);
        let ffd_count = |s: &str| s.chars().filter(|&c| c == '\u{FFFD}').count();
        if ffd_count(gb.as_ref()) <= ffd_count(&lossy_utf8) {
            gb.into_owned()
        } else {
            lossy_utf8.into_owned()
        }
    }
    #[cfg(not(windows))]
    {
        String::from_utf8_lossy(bytes).into_owned()
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[tokio::test]
    async fn test_process_spawn_and_wait() {
        let tool = ProcessTool::new();
        let cmd = "echo hello";

        // Submit
        let result = tool
            .execute(json!({
                "action": "submit",
                "session_id": "test_spawn_wait",
                "command": cmd,
            }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert!(v["pid"].as_u64().unwrap() > 0, "pid should be positive");
        assert_eq!(v["session_id"], "test_spawn_wait");

        // Wait for completion
        let result = tool
            .execute(json!({
                "action": "wait",
                "session_id": "test_spawn_wait",
            }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["exit_code"], 0, "exit code should be 0");
        assert_eq!(v["timed_out"], false, "should not time out");
    }

    #[tokio::test]
    async fn test_process_list_empty() {
        let tool = ProcessTool::new();
        let result = tool
            .execute(json!({ "action": "list" }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        let processes = v["processes"].as_array().unwrap();
        assert!(processes.is_empty(), "should have no processes");
    }

    #[tokio::test]
    async fn test_process_log_output() {
        let tool = ProcessTool::new();
        let cmd = "echo hello_world";

        // Submit
        tool.execute(json!({
            "action": "submit",
            "session_id": "test_log",
            "command": cmd,
        }))
        .await
        .unwrap();

        // Wait for the quick echo to finish
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let result = tool
                .execute(json!({
                    "action": "poll",
                    "session_id": "test_log",
                }))
                .await
                .unwrap();
            let v: Value = serde_json::from_str(&result).unwrap();
            if !v["running"].as_bool().unwrap() {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "timeout waiting for echo to finish"
            );
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        // Retrieve log output
        let result = tool
            .execute(json!({
                "action": "log",
                "session_id": "test_log",
            }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        let stdout = v["stdout"].as_str().unwrap();
        assert!(stdout.contains("hello_world"), "stdout should contain output");

        // Cleanup
        tool.execute(json!({
            "action": "wait",
            "session_id": "test_log",
        }))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_process_kill() {
        let tool = ProcessTool::new();
        let cmd = if cfg!(target_os = "windows") {
            "Start-Sleep -Seconds 10"
        } else {
            "sleep 10"
        };

        // Submit
        let result = tool
            .execute(json!({
                "action": "submit",
                "session_id": "test_kill",
                "command": cmd,
            }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        let pid = v["pid"].as_u64().unwrap();

        // Kill immediately
        let result = tool
            .execute(json!({
                "action": "kill",
                "session_id": "test_kill",
            }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["pid"], pid, "pid should match");
        let uptime = v["uptime_secs"].as_f64().unwrap();
        assert!(
            uptime < 10.0,
            "uptime ({}) should be less than the sleep duration (10s)",
            uptime
        );

        // Verify it's removed from the map
        let result = tool
            .execute(json!({ "action": "list" }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert!(v["processes"].as_array().unwrap().is_empty(), "should have no processes after kill");
    }

    #[tokio::test]
    async fn test_process_write_and_close_stdin() {
        // Create a process that reads from stdin and echoes back.
        let tool = ProcessTool::new();
        // On Unix: `cat` reads stdin line-by-line and echoes.
        // On Windows: `cmd /v /c "set /p s=&echo !s!"` reads one line via
        // `set /p` (which accepts pipe input) and echoes it with delayed
        // expansion, then cmd exits.
        let cmd = if cfg!(target_os = "windows") {
            "cmd /v /c \"set /p s=&echo !s!\""
        } else {
            "cat"
        };

        tool.execute(json!({
            "action": "submit",
            "session_id": "test_stdin",
            "command": cmd,
        }))
        .await
        .unwrap();

        // Write to stdin
        let result = tool
            .execute(json!({
                "action": "write",
                "session_id": "test_stdin",
                "data": "hello_stdin\r\n",
            }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert!(v["written"].as_u64().unwrap() > 0, "should have written bytes");

        // Close stdin so the process can exit
        let result = tool
            .execute(json!({
                "action": "close",
                "session_id": "test_stdin",
            }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["stdin_closed"], true);

        // Wait for exit (with a 10s safety timeout)
        let result = tool
            .execute(json!({
                "action": "wait",
                "session_id": "test_stdin",
                "timeout": 10,
            }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["timed_out"], false, "should not time out");
    }

    #[tokio::test]
    async fn test_process_invalid_session_id() {
        let tool = ProcessTool::new();

        // Poll with unknown session_id
        let result = tool
            .execute(json!({
                "action": "poll",
                "session_id": "nonexistent",
            }))
            .await;
        assert!(result.is_err(), "should error on unknown session_id");
        assert!(
            result.unwrap_err().to_string().contains("Unknown session_id"),
            "error should mention unknown session_id"
        );
    }

    #[tokio::test]
    async fn test_process_max_concurrent() {
        let tool = ProcessTool::new().with_max_concurrent(2);
        let cmd = if cfg!(target_os = "windows") {
            "Start-Sleep -Seconds 5"
        } else {
            "sleep 5"
        };

        // Spawn 2 processes (hits the limit)
        tool.execute(json!({
            "action": "submit",
            "session_id": "conc1",
            "command": cmd,
        }))
        .await
        .unwrap();

        tool.execute(json!({
            "action": "submit",
            "session_id": "conc2",
            "command": cmd,
        }))
        .await
        .unwrap();

        // Third should fail
        let result = tool
            .execute(json!({
                "action": "submit",
                "session_id": "conc3",
                "command": "echo should_not_run",
            }))
            .await;
        assert!(result.is_err(), "third submit should exceed max_concurrent");
        assert!(
            result.unwrap_err().to_string().contains("Maximum concurrent"),
            "error should mention limit"
        );

        // Cleanup
        tool.execute(json!({
            "action": "kill",
            "session_id": "conc1",
        }))
        .await
        .unwrap();
        tool.execute(json!({
            "action": "kill",
            "session_id": "conc2",
        }))
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn test_process_list_after_submit() {
        let tool = ProcessTool::new();
        let cmd = if cfg!(target_os = "windows") {
            "Start-Sleep -Seconds 3"
        } else {
            "sleep 3"
        };

        tool.execute(json!({
            "action": "submit",
            "session_id": "list_test",
            "command": cmd,
        }))
        .await
        .unwrap();

        let result = tool
            .execute(json!({ "action": "list" }))
            .await
            .unwrap();
        let v: Value = serde_json::from_str(&result).unwrap();
        let processes = v["processes"].as_array().unwrap();
        assert_eq!(processes.len(), 1, "should have one process");
        assert_eq!(processes[0]["session_id"], "list_test");
        assert!(processes[0]["running"].as_bool().unwrap(), "should be running");

        // Cleanup
        tool.execute(json!({
            "action": "kill",
            "session_id": "list_test",
        }))
        .await
        .unwrap();
    }
}
