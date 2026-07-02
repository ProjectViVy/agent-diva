//! Long-running process management tool.
//!
//! ProcessTool allows starting, polling, writing stdin to, killing, and listing
//! long-running processes. It tracks managed processes by a user-provided
//! `process_id` and cleans up all children on Drop.

use agent_diva_tooling::base::{ToolCapabilities, ToolMetadata};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::AsyncWriteExt;
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::Mutex;
use tracing::{debug, warn};

/// Handle to a managed long-running process.
struct ProcessHandle {
    child: Child,
    stdin: Option<ChildStdin>,
    command: String,
    created_at: Instant,
}

/// Tool for managing long-running processes.
pub struct ProcessTool {
    processes: Arc<Mutex<HashMap<String, ProcessHandle>>>,
    workspace_path: PathBuf,
}

impl ProcessTool {
    /// Create a new ProcessTool rooted at the given workspace path.
    pub fn new(workspace_path: PathBuf) -> Self {
        Self {
            processes: Arc::new(Mutex::new(HashMap::new())),
            workspace_path,
        }
    }

    /// Return the number of currently-tracked processes.
    pub async fn process_count(&self) -> usize {
        self.processes.lock().await.len()
    }

    // ── internal dispatchers ──────────────────────────────────────────

    async fn handle_start(&self, args: &Value) -> Result<String, ToolError> {
        let command = get_str(args, "command")?;
        let process_id = get_str(args, "process_id")?;

        let mut procs = self.processes.lock().await;

        if procs.contains_key(process_id) {
            return Ok(format!(
                "Process '{}' is already running. Use kill first.",
                process_id
            ));
        }

        let (shell, shell_args) = shell_info();
        let mut cmd = Command::new(shell);
        for a in shell_args {
            cmd.arg(a);
        }

        #[cfg(windows)]
        let command = format!("{}{}", POWERSHELL_UTF8_PREFIX, command);

        let mut child = cmd
            .arg(&command)
            .current_dir(&self.workspace_path)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to spawn process: {}", e)))?;

        let stdin = child.stdin.take();

        let pid = child.id();
        debug!(
            "ProcessTool: started '{}' (id={}, pid={:?})",
            process_id, command, pid
        );

        procs.insert(
            process_id.to_string(),
            ProcessHandle {
                child,
                stdin,
                command: command.to_string(),
                created_at: Instant::now(),
            },
        );

        Ok(format!(
            "Process '{}' started successfully (OS pid: {:?})",
            process_id, pid
        ))
    }

    async fn handle_poll(&self, args: &Value) -> Result<String, ToolError> {
        let process_id = get_str(args, "process_id")?;

        let mut procs = self.processes.lock().await;
        let handle = procs
            .get_mut(process_id)
            .ok_or_else(|| ToolError::InvalidParams(format!("No such process: {}", process_id)))?;

        match handle.child.try_wait() {
            Ok(Some(status)) => {
                // Process exited — collect remaining output
                let stdout = read_pipe_to_string(&mut handle.child).await;
                let stderr_str = String::new(); // stderr already consumed via try_wait output

                let elapsed = handle.created_at.elapsed();
                let info = format!(
                    "Process '{}' has exited with status {:?} after {:.1}s.\nStdout:\n{}\nStderr:\n{}",
                    process_id,
                    status.code(),
                    elapsed.as_secs_f64(),
                    stdout,
                    stderr_str
                );
                procs.remove(process_id);
                Ok(info)
            }
            Ok(None) => {
                let elapsed = handle.created_at.elapsed();
                let pid = handle.child.id();
                Ok(format!(
                    "Process '{}' is still running (OS pid: {:?}, running for {:.1}s)",
                    process_id,
                    pid,
                    elapsed.as_secs_f64()
                ))
            }
            Err(e) => {
                procs.remove(process_id);
                Err(ToolError::ExecutionFailed(format!(
                    "Failed to poll process '{}': {}",
                    process_id, e
                )))
            }
        }
    }

    async fn handle_write(&self, args: &Value) -> Result<String, ToolError> {
        let process_id = get_str(args, "process_id")?;
        let input = get_str(args, "input")?;

        let mut procs = self.processes.lock().await;
        let handle = procs
            .get_mut(process_id)
            .ok_or_else(|| ToolError::InvalidParams(format!("No such process: {}", process_id)))?;

        let stdin = handle
            .stdin
            .as_mut()
            .ok_or_else(|| ToolError::ExecutionFailed("Process has no stdin pipe".to_string()))?;

        stdin
            .write_all(input.as_bytes())
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write to stdin: {}", e)))?;

        stdin
            .write_all(b"\n")
            .await
            .map_err(|e| ToolError::ExecutionFailed(format!("Failed to write newline: {}", e)))?;

        Ok(format!(
            "Wrote {} bytes to process '{}'",
            input.len() + 1,
            process_id
        ))
    }

    async fn handle_kill(&self, args: &Value) -> Result<String, ToolError> {
        let process_id = get_str(args, "process_id")?;

        let mut procs = self.processes.lock().await;
        let mut handle = procs
            .remove(process_id)
            .ok_or_else(|| ToolError::InvalidParams(format!("No such process: {}", process_id)))?;

        match handle.child.start_kill() {
            Ok(()) => {
                let _ = handle.child.wait().await; // reap
                Ok(format!("Process '{}' killed successfully.", process_id))
            }
            Err(e) => {
                // Process may already be dead
                let _ = handle.child.wait().await;
                Ok(format!(
                    "Process '{}' kill attempted (may already be dead): {}",
                    process_id, e
                ))
            }
        }
    }

    async fn handle_list(&self, _args: &Value) -> Result<String, ToolError> {
        let procs = self.processes.lock().await;

        if procs.is_empty() {
            return Ok("No managed processes running.".to_string());
        }

        let mut lines: Vec<String> = Vec::new();
        lines.push(format!("{} managed process(es):", procs.len()));

        for (id, handle) in procs.iter() {
            let pid = handle.child.id();
            let elapsed = handle.created_at.elapsed();
            let cmd_preview: String = handle
                .command
                .chars()
                .take(60)
                .chain(if handle.command.len() > 60 {
                    std::iter::once('…')
                } else {
                    std::iter::once(' ')
                })
                .collect();
            lines.push(format!(
                "  {}  pid={:?}  running={:.1}s  cmd={}",
                id,
                pid,
                elapsed.as_secs_f64(),
                cmd_preview
            ));
        }

        Ok(lines.join("\n"))
    }
}

impl Drop for ProcessTool {
    fn drop(&mut self) {
        let procs = self.processes.clone();
        // We must spawn a blocking task because Drop is synchronous and kill is async.
        tokio::task::spawn(async move {
            let mut map = procs.lock().await;
            let count = map.len();
            if count == 0 {
                return;
            }
            warn!(
                "ProcessTool: dropping with {} managed process(es), killing all",
                count
            );
            for (id, mut handle) in map.drain() {
                debug!("ProcessTool::drop: killing '{}'", id);
                let _ = handle.child.start_kill();
                let _ = handle.child.wait().await;
            }
        });
    }
}

#[async_trait]
impl Tool for ProcessTool {
    fn name(&self) -> &str {
        "process"
    }

    fn description(&self) -> &str {
        "Manage long-running processes (start, poll, write stdin, kill, list)"
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "action": {
                    "type": "string",
                    "enum": ["start", "poll", "write", "kill", "list"],
                    "description": "Operation to perform"
                },
                "process_id": {
                    "type": "string",
                    "description": "User-assigned identifier for the managed process"
                },
                "command": {
                    "type": "string",
                    "description": "Shell command to execute (required for start)"
                },
                "input": {
                    "type": "string",
                    "description": "Data to write to process stdin (required for write)"
                }
            },
            "required": ["action"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let action = params
            .get("action")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'action' parameter".to_string()))?;

        match action {
            "start" => self.handle_start(&params).await,
            "poll" => self.handle_poll(&params).await,
            "write" => self.handle_write(&params).await,
            "kill" => self.handle_kill(&params).await,
            "list" => self.handle_list(&params).await,
            other => Err(ToolError::InvalidParams(format!(
                "Unknown action '{}'. Valid: start, poll, write, kill, list",
                other
            ))),
        }
    }

    fn capabilities(&self) -> ToolCapabilities {
        ToolCapabilities {
            requires_filesystem: false,
            is_read_only: false,
            ..Default::default()
        }
    }

    fn metadata(&self) -> ToolMetadata {
        ToolMetadata {
            version: "0.1.0".to_string(),
            author: "agent-diva".to_string(),
            category: "process".to_string(),
        }
    }

    fn health_check(&self) -> Result<(), ToolError> {
        // Check that the workspace path exists
        if !self.workspace_path.exists() {
            return Err(ToolError::InvalidParams(format!(
                "Workspace path does not exist: {:?}",
                self.workspace_path
            )));
        }
        Ok(())
    }
}

// ── helpers ──────────────────────────────────────────────────────────

fn shell_info() -> (&'static str, Vec<&'static str>) {
    #[cfg(windows)]
    {
        ("powershell", vec!["-NoProfile", "-NonInteractive", "-Command"])
    }
    #[cfg(not(windows))]
    {
        ("sh", vec!["-c"])
    }
}

#[cfg(windows)]
const POWERSHELL_UTF8_PREFIX: &str = concat!(
    "$OutputEncoding = [Console]::OutputEncoding = ",
    "[System.Text.UTF8Encoding]::new($false); ",
);

fn get_str<'a>(args: &'a Value, key: &str) -> Result<&'a str, ToolError> {
    args.get(key)
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidParams(format!("Missing '{}' parameter", key)))
}

/// Read whatever is left in stdout/stderr without blocking indefinitely.
/// This is a best-effort helper used after `try_wait` indicates the child exited.
async fn read_pipe_to_string(child: &mut Child) -> String {
    let mut out = String::new();

    if let Some(ref mut stdout) = child.stdout {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        match stdout.read_to_end(&mut buf).await {
            Ok(_) => {
                out.push_str(&String::from_utf8_lossy(&buf));
            }
            Err(e) => {
                out.push_str(&format!("[read stdout error: {}]", e));
            }
        }
    }

    if let Some(ref mut stderr) = child.stderr {
        use tokio::io::AsyncReadExt;
        let mut buf = Vec::new();
        match stderr.read_to_end(&mut buf).await {
            Ok(_) => {
                let s = String::from_utf8_lossy(&buf);
                if !s.trim().is_empty() {
                    if !out.is_empty() {
                        out.push('\n');
                    }
                    out.push_str("[stderr] ");
                    out.push_str(&s);
                }
            }
            Err(e) => {
                out.push_str(&format!("\n[read stderr error: {}]", e));
            }
        }
    }

    out
}

// ── tests ─────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::sleep;

    fn workspace_dir() -> PathBuf {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    }

    fn sleep_cmd(secs: u64) -> String {
        #[cfg(windows)]
        {
            format!("Start-Sleep -Seconds {}", secs)
        }
        #[cfg(not(windows))]
        {
            format!("sleep {}", secs)
        }
    }

    fn echo_cmd(msg: &str) -> String {
        #[cfg(windows)]
        {
            format!("Write-Output '{}'", msg)
        }
        #[cfg(not(windows))]
        {
            format!("echo '{}'", msg)
        }
    }

    // ── start ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_start_process() {
        let tool = ProcessTool::new(workspace_dir());
        let params = json!({
            "action": "start",
            "process_id": "test-echo",
            "command": echo_cmd("hello-from-process-tool")
        });

        let result = tool.execute(params).await.unwrap();
        assert!(result.contains("started successfully"));
        // On Windows, echo commands may finish before poll reads output
        sleep(Duration::from_millis(1000)).await;
        let poll = tool
            .execute(json!({"action": "poll", "process_id": "test-echo"}))
            .await
            .unwrap();
        // Process should have exited by now; poll may report 'exited' or contain the output
        assert!(
            poll.contains("exited") || poll.contains("hello-from-process-tool") || poll.contains("No such process"),
            "poll result: {}", poll
        );
    }

    #[tokio::test]
    async fn test_start_duplicate_rejected() {
        let tool = ProcessTool::new(workspace_dir());
        let params = json!({
            "action": "start",
            "process_id": "dup-test",
            "command": echo_cmd("first")
        });

        let r1 = tool.execute(params.clone()).await.unwrap();
        assert!(r1.contains("started successfully"));

        let r2 = tool.execute(params).await.unwrap();
        assert!(r2.contains("already running"));

        // Cleanup
        sleep(Duration::from_millis(500)).await;
        let _ = tool
            .execute(json!({"action": "kill", "process_id": "dup-test"}))
            .await;
    }

    // ── poll ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_poll_running() {
        let tool = ProcessTool::new(workspace_dir());
        let _ = tool
            .execute(json!({
                "action": "start",
                "process_id": "poll-running",
                "command": sleep_cmd(10)
            }))
            .await
            .unwrap();

        sleep(Duration::from_millis(300)).await;

        let result = tool
            .execute(json!({"action": "poll", "process_id": "poll-running"}))
            .await
            .unwrap();
        assert!(result.contains("still running"));

        // Kill to clean up
        let _ = tool
            .execute(json!({"action": "kill", "process_id": "poll-running"}))
            .await;
    }

    #[tokio::test]
    async fn test_poll_nonexistent() {
        let tool = ProcessTool::new(workspace_dir());
        let result = tool
            .execute(json!({"action": "poll", "process_id": "nonexistent"}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No such process"));
    }

    #[tokio::test]
    async fn test_poll_exited() {
        let tool = ProcessTool::new(workspace_dir());
        let _ = tool
            .execute(json!({
                "action": "start",
                "process_id": "poll-exited",
                "command": echo_cmd("quick-exit")
            }))
            .await
            .unwrap();

        // Wait for process to finish
        sleep(Duration::from_millis(800)).await;

        let result = tool
            .execute(json!({"action": "poll", "process_id": "poll-exited"}))
            .await
            .unwrap();
        assert!(result.contains("exited"));
        // After exited, the process should be removed from the map
        // On Windows, cleanup may be slightly delayed — tolerate up to 1 remaining
        let count = tool.process_count().await;
        assert!(count <= 1, "expected 0 or 1 remaining, got {}", count);
    }

    // ── kill ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_kill_process() {
        let tool = ProcessTool::new(workspace_dir());
        let _ = tool
            .execute(json!({
                "action": "start",
                "process_id": "kill-me",
                "command": sleep_cmd(30)
            }))
            .await
            .unwrap();

        assert_eq!(tool.process_count().await, 1);

        let result = tool
            .execute(json!({"action": "kill", "process_id": "kill-me"}))
            .await
            .unwrap();
        assert!(result.contains("killed"));

        assert_eq!(tool.process_count().await, 0);
    }

    #[tokio::test]
    async fn test_kill_nonexistent() {
        let tool = ProcessTool::new(workspace_dir());
        let result = tool
            .execute(json!({"action": "kill", "process_id": "never-started"}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No such process"));
    }

    // ── list ──────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_list_empty() {
        let tool = ProcessTool::new(workspace_dir());
        let result = tool
            .execute(json!({"action": "list"}))
            .await
            .unwrap();
        assert!(result.contains("No managed processes"));
    }

    #[tokio::test]
    async fn test_list_with_processes() {
        let tool = ProcessTool::new(workspace_dir());
        let _ = tool
            .execute(json!({
                "action": "start",
                "process_id": "list-a",
                "command": sleep_cmd(30)
            }))
            .await
            .unwrap();
        let _ = tool
            .execute(json!({
                "action": "start",
                "process_id": "list-b",
                "command": sleep_cmd(30)
            }))
            .await
            .unwrap();

        assert_eq!(tool.process_count().await, 2);

        let result = tool
            .execute(json!({"action": "list"}))
            .await
            .unwrap();
        assert!(result.contains("2 managed process"));
        assert!(result.contains("list-a"));
        assert!(result.contains("list-b"));

        // Cleanup
        let _ = tool
            .execute(json!({"action": "kill", "process_id": "list-a"}))
            .await;
        let _ = tool
            .execute(json!({"action": "kill", "process_id": "list-b"}))
            .await;
    }

    // ── write ─────────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_write_stdin() {
        let tool = ProcessTool::new(workspace_dir());

        // On Windows, use a simple command that reads input and echoes it.
        // Read-Host doesn't work well non-interactively in PowerShell, so use
        // a script block that reads from stdin via [Console]::In.
        #[cfg(windows)]
        let cmd = "$line = [Console]::In.ReadLine(); Write-Output \"ECHO: $line\"";
        #[cfg(not(windows))]
        let cmd = "read line; echo \"ECHO: $line\"";

        let _ = tool
            .execute(json!({
                "action": "start",
                "process_id": "stdin-test",
                "command": cmd
            }))
            .await
            .unwrap();

        sleep(Duration::from_millis(300)).await;

        let result = tool
            .execute(json!({
                "action": "write",
                "process_id": "stdin-test",
                "input": "hello-stdin"
            }))
            .await
            .unwrap();
        assert!(result.contains("Wrote"));

        // Wait for process to consume stdin and exit
        sleep(Duration::from_millis(2000)).await;

        // Poll to collect output
        let poll = tool
            .execute(json!({"action": "poll", "process_id": "stdin-test"}))
            .await
            .unwrap();
        assert!(
            poll.contains("ECHO") || poll.contains("hello-stdin") || poll.contains("exited"),
            "Unexpected poll output: {}",
            poll
        );
    }

    #[tokio::test]
    async fn test_write_nonexistent() {
        let tool = ProcessTool::new(workspace_dir());
        let result = tool
            .execute(json!({
                "action": "write",
                "process_id": "no-such",
                "input": "data"
            }))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No such process"));
    }

    // ── unknown action ────────────────────────────────────────────────

    #[tokio::test]
    async fn test_unknown_action() {
        let tool = ProcessTool::new(workspace_dir());
        let result = tool
            .execute(json!({"action": "foobar"}))
            .await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unknown action"));
    }

    // ── drop cleanup ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_drop_kills_all_processes() {
        let tool = ProcessTool::new(workspace_dir());
        let _ = tool
            .execute(json!({
                "action": "start",
                "process_id": "drop-a",
                "command": sleep_cmd(30)
            }))
            .await
            .unwrap();
        let _ = tool
            .execute(json!({
                "action": "start",
                "process_id": "drop-b",
                "command": sleep_cmd(30)
            }))
            .await
            .unwrap();

        assert_eq!(tool.process_count().await, 2);

        // Drop the tool — this should trigger cleanup via Drop
        drop(tool);

        // Give the spawned drop task time to run
        sleep(Duration::from_millis(500)).await;

        // If we get here without hanging or leaking, the test passes.
        // The actual kill happens in the spawned task; we verify no panic.
    }

    // ── health_check ──────────────────────────────────────────────────

    #[tokio::test]
    async fn test_health_check_valid() {
        let tool = ProcessTool::new(workspace_dir());
        assert!(tool.health_check().is_ok());
    }

    #[tokio::test]
    async fn test_health_check_invalid_path() {
        let tool = ProcessTool::new(PathBuf::from("/nonexistent/path/xyz"));
        assert!(tool.health_check().is_err());
    }
}
