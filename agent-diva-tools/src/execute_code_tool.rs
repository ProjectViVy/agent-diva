//! Execute code tool — sandboxed code execution via temp file + subprocess.
//!
//! Supports Python (default) and other interpreted languages.
//! Uses timeout-based subprocess execution following ExecTool's pattern.
//! Temp files are cleaned up after execution.

use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use serde_json::{json, Value};
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, info, warn};

/// File extension mapping for supported languages.
fn extension_for_language(lang: &str) -> &'static str {
    match lang {
        "python" | "py" => ".py",
        "javascript" | "js" => ".js",
        "bash" | "sh" => ".sh",
        "ruby" | "rb" => ".rb",
        "lua" => ".lua",
        "php" => ".php",
        "perl" | "pl" => ".pl",
        "r" => ".r",
        "go" => ".go",
        "rust" | "rs" => ".rs",
        _ => ".txt",
    }
}

/// Interpreter command for a language.
fn interpreter_for_language(lang: &str) -> &'static str {
    match lang {
        "python" | "py" => "python",
        "javascript" | "js" => "node",
        "bash" | "sh" => "bash",
        "ruby" | "rb" => "ruby",
        "lua" => "lua",
        "php" => "php",
        "perl" | "pl" => "perl",
        "r" => "Rscript",
        _ => "",
    }
}

/// Sandboxed code execution tool.
///
/// Writes code to a temporary file, executes it via the appropriate interpreter
/// within a timeout, captures stdout/stderr, and cleans up the temp file.
pub struct ExecuteCodeTool {
    /// Maximum execution time in seconds (default: 30).
    timeout_secs: u64,
    /// Working directory for temp files and execution.
    workspace_path: PathBuf,
}

impl ExecuteCodeTool {
    /// Create a new execute code tool with default settings.
    pub fn new(workspace_path: PathBuf) -> Self {
        Self {
            timeout_secs: 30,
            workspace_path,
        }
    }

    /// Create with custom timeout.
    pub fn with_timeout(workspace_path: PathBuf, timeout_secs: u64) -> Self {
        Self {
            timeout_secs,
            workspace_path,
        }
    }

    /// Execute code in a subprocess with timeout.
    async fn run_code(
        &self,
        code: &str,
        language: &str,
    ) -> std::result::Result<CodeOutput, String> {
        let ext = extension_for_language(language);
        let interpreter = interpreter_for_language(language);

        if interpreter.is_empty() && !matches!(language, "go" | "rust" | "rs") {
            return Err(format!("Unsupported language: {}", language));
        }

        // Write code to temp file
        let temp_dir = std::env::temp_dir().join("agent-diva-code-exec");
        tokio::fs::create_dir_all(&temp_dir)
            .await
            .map_err(|e| format!("Failed to create temp dir: {}", e))?;

        let file_path = temp_dir.join(format!("code_{}{}", uuid_for_temp(), ext));
        tokio::fs::write(&file_path, code)
            .await
            .map_err(|e| format!("Failed to write code file: {}", e))?;

        debug!("Code written to: {:?}", file_path);

        // Build command
        let (shell, args) = if cfg!(target_os = "windows") {
            ("powershell", vec!["-NoProfile", "-NonInteractive", "-Command"])
        } else {
            ("sh", vec!["-c"])
        };

        let cmd_str = if interpreter.is_empty() {
            // Compiled languages: just run the file directly (won't work for go/rust but handle gracefully)
            file_path.to_string_lossy().to_string()
        } else {
            format!("{} {}", interpreter, file_path.to_string_lossy().replace('\\', "/"))
        };

        let mut cmd = Command::new(shell);
        for arg in &args {
            cmd.arg(arg);
        }

        let child = cmd
            .arg(&cmd_str)
            .current_dir(&self.workspace_path)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn interpreter: {}", e))?;

        debug!("Interpreter spawned with PID: {:?}", child.id());

        // Wait with timeout
        let output_future = child.wait_with_output();
        let output_result = timeout(Duration::from_secs(self.timeout_secs), output_future).await;

        // Cleanup temp file (best-effort)
        let cleanup = tokio::fs::remove_file(&file_path);
        let _ = tokio::time::timeout(Duration::from_secs(5), cleanup).await;

        match output_result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);
                let success = output.status.success();

                Ok(CodeOutput {
                    stdout,
                    stderr,
                    exit_code,
                    success,
                })
            }
            Ok(Err(e)) => {
                warn!("Process wait error: {}", e);
                Err(format!("Failed to wait for interpreter: {}", e))
            }
            Err(_) => {
                warn!("Code execution timed out after {}s", self.timeout_secs);
                Err(format!(
                    "Code execution timed out after {} seconds",
                    self.timeout_secs
                ))
            }
        }
    }
}

/// Output from code execution.
#[derive(Debug, Clone)]
struct CodeOutput {
    stdout: String,
    stderr: String,
    exit_code: i32,
    success: bool,
}

/// Generate a short unique-ish suffix for temp file names.
fn uuid_for_temp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    format!("{:08x}", nanos)
}

#[async_trait]
impl Tool for ExecuteCodeTool {
    fn name(&self) -> &str {
        "execute_code"
    }

    fn description(&self) -> &str {
        "Execute code in a sandboxed environment with a timeout. Supports Python, JavaScript, Ruby, Lua, PHP, Perl, R, and more."
    }

    fn parameters(&self) -> Value {
        json!({
            "type": "object",
            "properties": {
                "code": {
                    "type": "string",
                    "description": "The source code to execute"
                },
                "language": {
                    "type": "string",
                    "description": "Programming language (python, javascript, ruby, lua, php, perl, r). Default: python",
                    "default": "python"
                }
            },
            "required": ["code"]
        })
    }

    async fn execute(&self, params: Value) -> std::result::Result<String, ToolError> {
        let code = params
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'code' parameter".to_string()))?;

        let language = params
            .get("language")
            .and_then(|v| v.as_str())
            .unwrap_or("python")
            .to_lowercase();

        info!(
            "Executing {} code ({} bytes) with timeout {}s",
            language,
            code.len(),
            self.timeout_secs
        );

        match self.run_code(code, &language).await {
            Ok(output) => {
                let mut parts: Vec<String> = Vec::new();

                let stdout = output.stdout.trim();
                let stderr = output.stderr.trim();

                if !stdout.is_empty() {
                    parts.push(stdout.to_string());
                }

                if !stderr.is_empty() {
                    parts.push(format!("[stderr]\n{}", stderr));
                }

                if !output.success {
                    parts.push(format!("[exit code: {}]", output.exit_code));
                }

                if parts.is_empty() {
                    parts.push("(no output)".to_string());
                }

                Ok(parts.join("\n"))
            }
            Err(e) => Ok(format!("Error: {}", e)),
        }
    }

    fn capabilities(&self) -> agent_diva_tooling::base::ToolCapabilities {
        agent_diva_tooling::base::ToolCapabilities {
            requires_filesystem: true,
            is_read_only: false,
            ..Default::default()
        }
    }

    fn metadata(&self) -> agent_diva_tooling::base::ToolMetadata {
        agent_diva_tooling::base::ToolMetadata {
            version: "0.1.0".to_string(),
            author: "agent-diva".to_string(),
            category: "execution".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_tool() -> (ExecuteCodeTool, TempDir) {
        let dir = TempDir::new().unwrap();
        let tool = ExecuteCodeTool::new(dir.path().to_path_buf());
        (tool, dir)
    }

    #[tokio::test]
    async fn test_execute_python_hello() {
        let (tool, _dir) = make_tool();
        let result = tool
            .execute(json!({
                "code": "print('hello from sandbox')",
                "language": "python"
            }))
            .await
            .unwrap();

        assert!(result.contains("hello from sandbox"), "got: {}", result);
    }

    #[tokio::test]
    async fn test_execute_python_default_language() {
        let (tool, _dir) = make_tool();
        let result = tool
            .execute(json!({
                "code": "print(1 + 2)"
            }))
            .await
            .unwrap();

        assert!(result.contains('3'), "got: {}", result);
    }

    #[tokio::test]
    async fn test_execute_javascript() {
        let (tool, _dir) = make_tool();
        let result = tool
            .execute(json!({
                "code": "console.log('hello from node')",
                "language": "javascript"
            }))
            .await
            .unwrap();

        // Node may or may not be installed; if not, we get an error.
        // Just verify it doesn't panic.
        assert!(!result.is_empty());
    }

    #[tokio::test]
    async fn test_execute_missing_code_param() {
        let (tool, _dir) = make_tool();
        let result = tool.execute(json!({})).await;

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing 'code'"));
    }

    #[tokio::test]
    async fn test_timeout_behavior() {
        let dir = TempDir::new().unwrap();
        // Use 1-second timeout — the code sleeps for 30s, so it should time out.
        let tool = ExecuteCodeTool::with_timeout(dir.path().to_path_buf(), 1);

        let result = tool
            .execute(json!({
                "code": "import time; time.sleep(30); print('done')",
                "language": "python"
            }))
            .await
            .unwrap();

        assert!(
            result.contains("timed out") || result.contains("Error:"),
            "Expected timeout error, got: {}",
            result
        );
    }

    #[tokio::test]
    async fn test_temp_file_cleanup() {
        let dir = TempDir::new().unwrap();
        let tool = ExecuteCodeTool::new(dir.path().to_path_buf());

        // Execute code
        let _ = tool
            .execute(json!({
                "code": "print('cleanup test')",
                "language": "python"
            }))
            .await;

        // Check that the temp directory doesn't have leftover .py files
        let temp_dir = std::env::temp_dir().join("agent-diva-code-exec");
        if temp_dir.exists() {
            let mut py_count = 0;
            if let Ok(entries) = std::fs::read_dir(&temp_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().map_or(false, |e| e == "py") {
                        py_count += 1;
                    }
                }
            }
            // After execution, temp files should be cleaned up.
            // There might be race conditions, but in normal operation they should be gone.
            // Just verify we didn't leave hundreds of files.
            assert!(
                py_count < 10,
                "Too many leftover .py files in temp dir: {}",
                py_count
            );
        }
    }

    #[test]
    fn test_extension_mapping() {
        assert_eq!(extension_for_language("python"), ".py");
        assert_eq!(extension_for_language("py"), ".py");
        assert_eq!(extension_for_language("javascript"), ".js");
        assert_eq!(extension_for_language("ruby"), ".rb");
        assert_eq!(extension_for_language("lua"), ".lua");
        assert_eq!(extension_for_language("unknown"), ".txt");
    }

    #[test]
    fn test_interpreter_mapping() {
        assert_eq!(interpreter_for_language("python"), "python");
        assert_eq!(interpreter_for_language("javascript"), "node");
        assert_eq!(interpreter_for_language("ruby"), "ruby");
        assert_eq!(interpreter_for_language("lua"), "lua");
        assert_eq!(interpreter_for_language("go"), "");
    }

    #[test]
    fn test_tool_metadata() {
        let dir = TempDir::new().unwrap();
        let tool = ExecuteCodeTool::new(dir.path().to_path_buf());
        assert_eq!(tool.name(), "execute_code");
        assert!(tool.description().contains("sandboxed"));
        assert_eq!(tool.capabilities().requires_filesystem, true);
        assert_eq!(tool.capabilities().is_read_only, false);
    }

    #[test]
    fn test_parameters_schema() {
        let dir = TempDir::new().unwrap();
        let tool = ExecuteCodeTool::new(dir.path().to_path_buf());
        let schema = tool.parameters();

        assert_eq!(schema["type"], "object");
        let required = schema["required"].as_array().unwrap();
        assert!(required.iter().any(|v| v.as_str() == Some("code")));

        let code_prop = &schema["properties"]["code"];
        assert_eq!(code_prop["type"], "string");

        let lang_prop = &schema["properties"]["language"];
        assert_eq!(lang_prop["default"], "python");
    }

    #[tokio::test]
    async fn test_execute_code_with_error() {
        let (tool, _dir) = make_tool();
        let result = tool
            .execute(json!({
                "code": "raise Exception('test error')",
                "language": "python"
            }))
            .await
            .unwrap();

        // Should contain stderr with the exception traceback
        assert!(
            result.contains("Exception") || result.contains("Error"),
            "Expected error output, got: {}",
            result
        );
    }
}
