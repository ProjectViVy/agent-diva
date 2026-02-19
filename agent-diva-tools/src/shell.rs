//! Shell execution tool

use crate::base::{Tool, ToolError};
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;

/// Shell execution tool
pub struct ExecTool {
    timeout_secs: u64,
    working_dir: Option<PathBuf>,
    deny_patterns: Vec<Regex>,
    allow_patterns: Vec<Regex>,
    restrict_to_workspace: bool,
}

impl ExecTool {
    /// Create a new exec tool with default settings
    pub fn new() -> Self {
        Self {
            timeout_secs: 60,
            working_dir: None,
            deny_patterns: Self::default_deny_patterns(),
            allow_patterns: Vec::new(),
            restrict_to_workspace: false,
        }
    }

    /// Create with custom settings
    pub fn with_config(
        timeout_secs: u64,
        working_dir: Option<PathBuf>,
        restrict_to_workspace: bool,
    ) -> Self {
        Self {
            timeout_secs,
            working_dir,
            deny_patterns: Self::default_deny_patterns(),
            allow_patterns: Vec::new(),
            restrict_to_workspace,
        }
    }

    /// Default dangerous command patterns
    fn default_deny_patterns() -> Vec<Regex> {
        vec![
            r"\brm\s+-[rf]{1,2}\b",            // rm -r, rm -rf
            r"\bdel\s+/[fq]\b",                // del /f, del /q
            r"\brmdir\s+/s\b",                 // rmdir /s
            r"\b(format|mkfs|diskpart)\b",     // disk operations
            r"\bdd\s+if=",                     // dd
            r">\s*/dev/sd",                    // write to disk
            r"\b(shutdown|reboot|poweroff)\b", // system power
            r":\(\)\s*\{.*\};\s*:",            // fork bomb
        ]
        .into_iter()
        .filter_map(|p| Regex::new(p).ok())
        .collect()
    }

    /// Guard command against dangerous patterns
    fn guard_command(&self, command: &str, cwd: &Path) -> Result<(), String> {
        let cmd = command.trim();
        let lower = cmd.to_lowercase();

        // Check deny patterns
        for pattern in &self.deny_patterns {
            if pattern.is_match(&lower) {
                return Err(
                    "Command blocked by safety guard (dangerous pattern detected)".to_string(),
                );
            }
        }

        // Check allow patterns (if any)
        if !self.allow_patterns.is_empty() {
            let allowed = self.allow_patterns.iter().any(|p| p.is_match(&lower));
            if !allowed {
                return Err("Command blocked by safety guard (not in allowlist)".to_string());
            }
        }

        // Check workspace restriction
        if self.restrict_to_workspace {
            if cmd.contains("..\\") || cmd.contains("../") {
                return Err("Command blocked by safety guard (path traversal detected)".to_string());
            }

            // Check absolute paths in command
            let cwd_canonical = cwd.canonicalize().unwrap_or_else(|_| cwd.to_path_buf());

            // Windows paths: C:\...
            let win_pattern = r"[A-Za-z]:";
            let win_re = Regex::new(win_pattern).unwrap();
            // POSIX paths: /...
            let posix_pattern = r"/[^\s]+";
            let posix_re = Regex::new(posix_pattern).unwrap();

            for cap in win_re.find_iter(cmd).chain(posix_re.find_iter(cmd)) {
                let path_str = cap.as_str();
                if let Ok(p) = Path::new(path_str).canonicalize() {
                    if !p.starts_with(&cwd_canonical) && p != cwd_canonical {
                        return Err("Command blocked by safety guard (path outside working dir)"
                            .to_string());
                    }
                }
            }
        }

        Ok(())
    }
}

impl Default for ExecTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for ExecTool {
    fn name(&self) -> &str {
        "exec"
    }

    fn description(&self) -> &str {
        "Execute a shell command and return its output. Use with caution."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "command": {
                    "type": "string",
                    "description": "The shell command to execute"
                },
                "working_dir": {
                    "type": "string",
                    "description": "Optional working directory for the command"
                }
            },
            "required": ["command"]
        })
    }

    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'command' parameter".to_string()))?;

        let working_dir = params
            .get("working_dir")
            .and_then(|v| v.as_str())
            .map(PathBuf::from)
            .or_else(|| self.working_dir.clone())
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // Safety guard
        if let Err(err) = self.guard_command(command, &working_dir) {
            return Ok(format!("Error: {}", err));
        }

        // Execute command
        let result = self.execute_command(command, &working_dir).await;

        match result {
            Ok(output) => Ok(output),
            Err(err) => Ok(format!("Error executing command: {}", err)),
        }
    }
}

impl ExecTool {
    /// Execute the command and return output
    async fn execute_command(&self, command: &str, cwd: &Path) -> Result<String, String> {
        // Determine shell based on OS
        let (shell, shell_arg) = if cfg!(target_os = "windows") {
            ("cmd", "/C")
        } else {
            ("sh", "-c")
        };

        let child = Command::new(shell)
            .arg(shell_arg)
            .arg(command)
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        // Wait for output with timeout
        let output_future = child.wait_with_output();
        let output_result = timeout(Duration::from_secs(self.timeout_secs), output_future).await;

        let output = match output_result {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(format!("Failed to wait for process: {}", e)),
            Err(_) => {
                return Err(format!(
                    "Command timed out after {} seconds",
                    self.timeout_secs
                ))
            }
        };

        let mut result_parts = Vec::new();

        // Stdout
        if !output.stdout.is_empty() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            result_parts.push(stdout);
        }

        // Stderr
        if !output.stderr.is_empty() {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            if !stderr.trim().is_empty() {
                result_parts.push(format!("STDERR:\n{}", stderr));
            }
        }

        // Exit code
        if !output.status.success() {
            result_parts.push(format!(
                "\nExit code: {}",
                output.status.code().unwrap_or(-1)
            ));
        }

        let mut result = if result_parts.is_empty() {
            "(no output)".to_string()
        } else {
            result_parts.join("\n")
        };

        // Truncate very long output
        const MAX_LEN: usize = 10000;
        if result.len() > MAX_LEN {
            let truncated = result.chars().take(MAX_LEN).collect::<String>();
            result = format!(
                "{}\n... (truncated, {} more chars)",
                truncated,
                result.len() - MAX_LEN
            );
        }

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_exec_simple_command() {
        let tool = ExecTool::new();
        let params = json!({
            "command": if cfg!(target_os = "windows") { "echo hello" } else { "echo hello" }
        });

        let result = tool.execute(params).await.unwrap();
        assert!(result.contains("hello"));
    }

    #[tokio::test]
    async fn test_exec_blocked_command() {
        let tool = ExecTool::new();
        let params = json!({
            "command": "rm -rf /"
        });

        let result = tool.execute(params).await.unwrap();
        assert!(result.contains("blocked by safety guard"));
    }

    #[tokio::test]
    async fn test_exec_timeout() {
        let tool = ExecTool::with_config(1, None, false);
        let params = json!({
            "command": if cfg!(target_os = "windows") {
                // `timeout /t` may exit immediately under piped I/O (e.g. CI),
                // so use ping as a portable-ish sleep for cmd.exe.
                "ping -n 6 127.0.0.1 > NUL"
            } else {
                "sleep 5"
            }
        });

        let result = tool.execute(params).await.unwrap();
        assert!(result.contains("timed out"));
    }

    #[test]
    fn test_guard_dangerous_patterns() {
        let tool = ExecTool::new();
        let cwd = PathBuf::from(".");

        assert!(tool.guard_command("rm -rf /tmp", &cwd).is_err());
        assert!(tool.guard_command("del /f file.txt", &cwd).is_err());
        assert!(tool.guard_command("shutdown -h now", &cwd).is_err());
    }

    #[test]
    fn test_guard_safe_commands() {
        let tool = ExecTool::new();
        let cwd = PathBuf::from(".");

        assert!(tool.guard_command("ls -la", &cwd).is_ok());
        assert!(tool.guard_command("echo hello", &cwd).is_ok());
        assert!(tool.guard_command("cat file.txt", &cwd).is_ok());
    }
}
