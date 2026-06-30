//! Python code execution tool
//!
//! Executes Python code in a subprocess with timeout, output limits,
//! and process kill guarantees. This is a one-shot execution tool —
//! each invocation spawns a dedicated Python process and cleans it up.

use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, info};

/// Execute Python code in a subprocess with timeout and output limits.
pub struct ExecuteCodeTool {
    python_binary: String,
    timeout_secs: u64,
    max_output_chars: usize,
}

impl ExecuteCodeTool {
    /// Create a new `ExecuteCodeTool` with default settings.
    ///
    /// - Python binary is auto-detected via the `which` crate.
    /// - Timeout: 60 seconds.
    /// - Max output: 10 000 characters.
    pub fn new() -> Self {
        let python_binary = resolve_python_binary();
        Self {
            python_binary,
            timeout_secs: 60,
            max_output_chars: 10_000,
        }
    }

    /// Create an `ExecuteCodeTool` with custom settings.
    ///
    /// Pass `None` for `python_binary` to auto-detect.
    pub fn with_config(
        python_binary: Option<String>,
        timeout_secs: u64,
        max_output_chars: usize,
    ) -> Self {
        let python_binary = python_binary.unwrap_or_else(resolve_python_binary);
        Self {
            python_binary,
            timeout_secs,
            max_output_chars,
        }
    }

    /// Execute Python code and return sanitized output.
    async fn execute_code(&self, code: &str) -> Result<String, String> {
        info!(
            "Executing Python code ({} chars, {}s timeout)",
            code.len(),
            self.timeout_secs
        );
        debug!("Python binary: {}", self.python_binary);

        let mut cmd = Command::new(&self.python_binary);
        cmd.arg("-c")
            .arg(code)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        let child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn Python process: {}", e))?;

        let pid = child.id().unwrap_or(0);
        debug!("Python process spawned with PID: {}", pid);

        // Wait for output with timeout
        let output_result = timeout(
            Duration::from_secs(self.timeout_secs),
            child.wait_with_output(),
        )
        .await;

        let output = match output_result {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => {
                return Err(format!("Failed to read process output: {}", e));
            }
            Err(_elapsed) => {
                // Timeout — kill the process by PID since `wait_with_output`
                // consumes the `Child`.
                debug!(
                    "Python process timed out after {}s, killing PID {}",
                    self.timeout_secs, pid
                );
                kill_process(pid).await;
                return Err(format!(
                    "Execution timed out after {} seconds",
                    self.timeout_secs
                ));
            }
        };

        let mut result_parts: Vec<String> = Vec::new();

        // Stdout
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        if !stdout.is_empty() {
            result_parts.push(stdout);
        }

        // Stderr — prefix each line so callers can distinguish it from stdout
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if !stderr.trim().is_empty() {
            let formatted: Vec<String> =
                stderr.lines().map(|line| format!("[stderr] {}", line)).collect();
            result_parts.push(formatted.join("\n"));
        }

        // Non-zero exit code
        if !output.status.success() {
            let exit_code = output.status.code().unwrap_or(-1);
            result_parts.push(format!("\nExit code: {}", exit_code));
        }

        let mut result = if result_parts.is_empty() {
            String::new()
        } else {
            result_parts.join("\n")
        };

        // Enforce output character limit
        if result.len() > self.max_output_chars {
            let truncated: String = result.chars().take(self.max_output_chars).collect();
            result = format!(
                "{}\n... (truncated, {} more chars)",
                truncated,
                result.len() - self.max_output_chars
            );
        }

        Ok(result)
    }
}

impl Default for ExecuteCodeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ExecuteCodeTool {
    fn name(&self) -> &str {
        "execute_code"
    }

    fn description(&self) -> &str {
        "Execute Python code in a subprocess with timeout and output limits. \
         Use for Python scripts, calculations, data processing, and any Python code execution. \
         The code is passed via -c flag to the Python interpreter."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "The Python code to execute"
                },
                "language": {
                    "type": "string",
                    "enum": ["python"],
                    "description": "The language to use (only Python is supported)",
                    "default": "python"
                }
            },
            "required": ["code"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let code = params
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ToolError::InvalidParams("Missing required 'code' parameter".to_string())
            })?;

        // Optional language parameter — early-reject unsupported values
        if let Some(lang) = params.get("language").and_then(|v| v.as_str()) {
            if lang != "python" {
                return Ok(format!(
                    "Unsupported language '{}'. Only 'python' is supported.",
                    lang
                ));
            }
        }

        match self.execute_code(code).await {
            Ok(output) => Ok(output),
            Err(err) => Ok(format!("Error executing Python code: {}", err)),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Resolve a usable Python binary name via `which`, falling back to a
/// platform-appropriate default.
fn resolve_python_binary() -> String {
    which::which("python")
        .or_else(|_| which::which("python3"))
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|_| {
            if cfg!(windows) {
                "python".to_string()
            } else {
                "python3".to_string()
            }
        })
}

/// Force-kill a process by PID.
///
/// On Unix this sends SIGKILL via the `kill` command; on Windows it uses
/// `taskkill /F /PID`.
async fn kill_process(pid: u32) {
    if pid == 0 {
        return;
    }

    #[cfg(unix)]
    {
        let _ = std::process::Command::new("kill")
            .arg("-9")
            .arg(pid.to_string())
            .output();
    }

    #[cfg(windows)]
    {
        let _ = tokio::process::Command::new("taskkill")
            .args(["/F", "/PID", &pid.to_string()])
            .output()
            .await;
    }

    // On other platforms we have no reliable way to force-kill; just abandon
    // the zombie (the process table will eventually reap it when the child
    // terminates on its own or the OS cleans up).
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_execute_code_hello() {
        let tool = ExecuteCodeTool::new();
        let params = json!({"code": "print('hello')"});
        let result = tool.execute(params).await.unwrap();
        assert!(result.contains("hello"), "expected 'hello' in output, got: {:?}", result);
    }

    #[tokio::test]
    async fn test_execute_code_compute() {
        let tool = ExecuteCodeTool::new();
        let params = json!({"code": "print(6 * 7)"});
        let result = tool.execute(params).await.unwrap();
        assert!(result.contains("42"), "expected '42' in output, got: {:?}", result);
    }

    #[tokio::test]
    async fn test_execute_code_timeout() {
        // 1-second timeout for a 10-second sleep
        let tool = ExecuteCodeTool::with_config(None, 1, 10_000);
        let params = json!({"code": "import time; time.sleep(10)"});
        let result = tool.execute(params).await.unwrap();
        assert!(
            result.to_lowercase().contains("timed out"),
            "expected timeout message, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_execute_code_syntax_error() {
        let tool = ExecuteCodeTool::new();
        let params = json!({"code": "print(undefined_var)"});
        let result = tool.execute(params).await.unwrap();
        // Python emits a NameError on stderr
        let has_error = result.contains("NameError") || result.contains("[stderr]");
        assert!(has_error, "expected NameError or stderr output, got: {:?}", result);
    }

    #[tokio::test]
    async fn test_execute_code_empty() {
        let tool = ExecuteCodeTool::new();
        let params = json!({"code": ""});
        let result = tool.execute(params).await.unwrap();
        // Empty code should either produce empty output or a Python error
        assert!(
            result.trim().is_empty() || result.contains("Error"),
            "expected empty or error output, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_execute_code_truncation() {
        // 20 KB of output with a 1 KB limit
        let tool = ExecuteCodeTool::with_config(None, 30, 1024);
        let params = json!({"code": "print('x' * 20000)"});
        let result = tool.execute(params).await.unwrap_or_default();
        // Output must be <= max_output_chars + truncation suffix
        assert!(
            result.len() <= 1200 || result.contains("truncated"),
            "expected truncated output, result len = {}",
            result.len()
        );
    }

    #[tokio::test]
    async fn test_execute_code_multiline() {
        let tool = ExecuteCodeTool::new();
        let params = json!({"code": "for i in range(3):\n    print(i)"});
        let result = tool.execute(params).await.unwrap();
        assert!(
            result.contains("0") && result.contains("1"),
            "expected '0' and '1' in output, got: {:?}",
            result
        );
    }

    #[tokio::test]
    async fn test_execute_code_language_rejection() {
        let tool = ExecuteCodeTool::new();
        let params = json!({"code": "print('hi')", "language": "javascript"});
        let result = tool.execute(params).await.unwrap();
        assert!(
            result.contains("Unsupported language"),
            "expected unsupported language error, got: {:?}",
            result
        );
    }
}
