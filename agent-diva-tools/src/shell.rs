//! Shell execution tool

use crate::sanitize::sanitize_for_json;
use agent_diva_sandbox::{
    AskForApproval, CommandApprovalCoordinator, CommandApprovalKey, CommandApprovalScope,
    CommandApprovalStatus, DefaultGuardianReviewer, ExecPolicyManager, GuardianConfig,
    GuardianManager, ReviewDecision, SandboxConfig, SandboxError, SandboxManager, ToolOrchestrator,
};
use agent_diva_tooling::{Tool, ToolError};
use async_trait::async_trait;
use regex::Regex;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use tokio::process::Command;
use tokio::time::timeout;
use tracing::{debug, info};

/// Decode bytes captured from a child process pipe.
/// On Windows, PowerShell and cmd often emit system ANSI (e.g. GBK on zh-CN); treating that as
/// UTF-8 produces U+FFFD garbage and can break downstream LLM JSON. Prefer strict UTF-8, then
/// GB18030 when it yields fewer replacement characters than UTF-8 lossy decoding.
fn decode_shell_pipe_bytes(bytes: &[u8]) -> String {
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

#[cfg(windows)]
const POWERSHELL_UTF8_OUTPUT_PREFIX: &str = concat!(
    "$OutputEncoding = [Console]::OutputEncoding = ",
    "[System.Text.UTF8Encoding]::new($false); ",
);

/// Shell execution tool
pub struct ExecTool {
    timeout_secs: u64,
    working_dir: Option<PathBuf>,
    deny_patterns: Vec<Regex>,
    allow_patterns: Vec<Regex>,
    restrict_to_workspace: bool,
    orchestrator: Option<Arc<ToolOrchestrator>>,
    approval_coordinator: Option<CommandApprovalCoordinator>,
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
            orchestrator: None,
            approval_coordinator: None,
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
            orchestrator: None,
            approval_coordinator: None,
        }
    }

    /// Route production execution through the sandbox orchestrator.
    ///
    /// `approval_policy` controls when the orchestrator prompts the user:
    /// - `OnFailure` (default): only after sandbox execution fails.
    /// - `OnRequest`: before every tool call ("cautious" mode in the GUI).
    /// - `UnlessTrusted`: before every non-trusted command.
    /// - `Never`: never prompt.
    pub fn with_approval_backend(
        mut self,
        coordinator: Option<CommandApprovalCoordinator>,
        approval_policy: AskForApproval,
    ) -> Self {
        let workspace = self
            .working_dir
            .clone()
            .unwrap_or_else(|| PathBuf::from("."));
        let mut config = SandboxConfig::workspace_write(workspace);
        config.timeout_seconds = self.timeout_secs;
        let manager = Arc::new(SandboxManager::new(&config));
        if approval_policy == AskForApproval::Never {
            self.orchestrator = Some(Arc::new(ToolOrchestrator::new(manager, approval_policy)));
        } else {
            // Attach a mode-driven Guardian so smart/cautious/trusted produce
            // distinct approval behavior in production. Known-safe detection
            // reuses the coordinator's rule store.
            let rules = coordinator
                .as_ref()
                .and_then(CommandApprovalCoordinator::command_rules)
                .cloned();
            let guardian_config = GuardianConfig::for_ask(approval_policy);
            let reviewer = Arc::new(DefaultGuardianReviewer::with_rules(
                approval_policy,
                rules.clone(),
            ));
            let guardian = GuardianManager::new(guardian_config, reviewer);
            let exec_policy = rules.map(|store| {
                let guardian_path = store
                    .path()
                    .parent()
                    .map(|dir| dir.join("execpolicy-guardian.toml"))
                    .unwrap_or_else(|| PathBuf::from("execpolicy-guardian.toml"));
                Arc::new(
                    ExecPolicyManager::from_command_rule_store(&store)
                        .with_rules_path(guardian_path),
                )
            });
            self.orchestrator = Some(Arc::new(
                ToolOrchestrator::new(manager, approval_policy)
                    .with_guardian_and_exec_policy(Arc::new(guardian), exec_policy),
            ));
        }
        self.approval_coordinator = coordinator;
        self
    }

    #[cfg(test)]
    fn with_orchestrator(
        mut self,
        orchestrator: Arc<ToolOrchestrator>,
        coordinator: Option<CommandApprovalCoordinator>,
    ) -> Self {
        self.orchestrator = Some(orchestrator);
        self.approval_coordinator = coordinator;
        self
    }

    #[cfg(test)]
    fn has_guardian_for_test(&self) -> bool {
        self.orchestrator
            .as_ref()
            .map(|o| o.has_guardian())
            .unwrap_or(false)
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

            // Windows absolute paths: C:\... (quoted or unquoted)
            let win_pattern = r#"(?i)[A-Za-z]:\\[^\s"']+"#;
            let win_re = Regex::new(win_pattern).unwrap();
            // POSIX absolute paths: /...
            let posix_pattern = r#"/[^\s"']+"#;
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

/// Wave C2 workspace-root enforcement.
///
/// Reject any resolved working directory that does not reside at or inside
/// the tool's configured workspace scope after canonicalization. Symlinks and
/// `..` traversal are resolved so naive string comparisons cannot be fooled.
///
/// If the target path does not yet exist, `canonicalize` is attempted on the
/// parent chain; if that also fails we fall back to the raw path and compare
/// normalized string components, which is safe because the workspace scope
/// itself must already exist when the tool is configured.
fn enforce_workspace_boundary(target: &Path, workspace_scope: &Path) -> Result<(), String> {
    let resolved = std::fs::canonicalize(target)
        .or_else(|_| canonicalize_best_effort(target))
        .unwrap_or_else(|_| target.to_path_buf());
    let scope =
        std::fs::canonicalize(workspace_scope).unwrap_or_else(|_| workspace_scope.to_path_buf());

    if resolved.starts_with(&scope) {
        Ok(())
    } else {
        Err(format!(
            "resolved path `{}` is outside workspace scope `{}`",
            resolved.display(),
            scope.display()
        ))
    }
}

/// Best-effort canonicalize for paths whose tail may not yet exist.
fn canonicalize_best_effort(path: &Path) -> std::io::Result<PathBuf> {
    let mut probe = path.to_path_buf();
    let mut tail = Vec::new();
    while !probe.as_os_str().is_empty() {
        if let Ok(base) = std::fs::canonicalize(&probe) {
            tail.reverse();
            let mut rebuilt = base;
            for component in tail {
                rebuilt.push(component);
            }
            return Ok(rebuilt);
        }
        if let Some(file_name) = probe.file_name() {
            tail.push(file_name.to_os_string());
        }
        if !probe.pop() {
            break;
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::NotFound,
        "no existing ancestor for canonicalize",
    ))
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

    fn timeout_secs(&self) -> Option<u64> {
        self.orchestrator
            .as_ref()
            .map(|_| self.timeout_secs.saturating_add(315))
    }

    async fn execute(&self, params: Value) -> Result<String, ToolError> {
        let command = params
            .get("command")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ToolError::InvalidParams("Missing 'command' parameter".to_string()))?;

        // Resolve working_dir with Wave C2 workspace-root enforcement.
        //
        // Priority: model-supplied `working_dir` > tool-config default > workspace root
        // (when the tool is workspace-scoped) > process CWD (legacy fallback for
        // unscoped tools).
        //
        // When the tool is scoped to a workspace root, any resolved directory must
        // remain inside that root after canonicalization; otherwise the command is
        // rejected with an explanatory message. This prevents the model from
        // pivoting shell execution outside the active project.
        let (requested, from_model) = params
            .get("working_dir")
            .and_then(|v| v.as_str())
            .map(|s| (PathBuf::from(s), true))
            .unwrap_or_else(|| {
                self.working_dir
                    .clone()
                    .map(|p| (p, false))
                    .unwrap_or_else(|| {
                        (
                            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
                            false,
                        )
                    })
            });

        let working_dir = if requested.is_absolute() {
            requested.clone()
        } else if let Some(scope) = self.working_dir.as_ref() {
            scope.join(&requested)
        } else {
            requested.clone()
        };

        if let Some(scope) = self.working_dir.as_ref() {
            if let Err(err) = enforce_workspace_boundary(&working_dir, scope) {
                let source = if from_model {
                    "model-supplied working_dir"
                } else {
                    "configured working_dir"
                };
                return Ok(format!(
                    "Error: {source} rejected: {err} (workspace root: {})",
                    scope.display()
                ));
            }
        }

        // Safety guard
        if let Err(err) = self.guard_command(command, &working_dir) {
            return Ok(format!("Error: {}", err));
        }

        let result = if let Some(orchestrator) = &self.orchestrator {
            self.execute_orchestrated(orchestrator, command, &working_dir, &params)
                .await
        } else {
            self.execute_command(command, &working_dir).await
        };

        match result {
            Ok(output) => Ok(output),
            Err(err) => Ok(format!("Error executing command: {}", err)),
        }
    }
}

impl ExecTool {
    async fn execute_orchestrated(
        &self,
        orchestrator: &ToolOrchestrator,
        command: &str,
        cwd: &Path,
        params: &Value,
    ) -> Result<String, String> {
        match orchestrator.run(command, &cwd.to_path_buf()).await {
            Ok(result) => Ok(sanitize_for_json(&result.output)),
            Err(SandboxError::ApprovalRequired { reason }) => {
                let coordinator = self.approval_coordinator.as_ref().ok_or_else(|| {
                    "sandbox escalation requires approval, but no approval client is available"
                        .to_string()
                })?;
                let scope = approval_scope_from_params(params)?;
                let (_, status) = coordinator
                    .request(command.to_string(), cwd.to_path_buf(), reason, scope)
                    .await;
                if status != CommandApprovalStatus::Approved {
                    return Err(format!("command approval ended with status {status:?}"));
                }
                orchestrator.sandbox_manager().record_approval(
                    CommandApprovalKey::new(command.to_string(), cwd.to_path_buf()),
                    ReviewDecision::ApprovedOnce,
                );
                orchestrator
                    .run(command, &cwd.to_path_buf())
                    .await
                    .map(|result| sanitize_for_json(&result.output))
                    .map_err(|error| error.to_string())
            }
            Err(error) => Err(error.to_string()),
        }
    }

    /// Execute the command and return output
    async fn execute_command(&self, command: &str, cwd: &Path) -> Result<String, String> {
        info!("Executing command: '{}' in {:?}", command, cwd);

        // Determine shell based on OS
        let (shell, args) = if cfg!(target_os = "windows") {
            (
                "powershell",
                vec!["-NoProfile", "-NonInteractive", "-Command"],
            )
        } else {
            ("sh", vec!["-c"])
        };

        #[cfg(windows)]
        let command = format!("{}{}", POWERSHELL_UTF8_OUTPUT_PREFIX, command);

        let mut cmd = Command::new(shell);
        for arg in args {
            cmd.arg(arg);
        }

        let child = cmd
            .arg(command)
            .current_dir(cwd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        debug!("Command spawned with PID: {:?}", child.id());

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

        let stdout_s = decode_shell_pipe_bytes(&output.stdout);
        let stderr_s = decode_shell_pipe_bytes(&output.stderr);

        // Stdout - sanitize to remove control characters and ANSI sequences
        if !output.stdout.is_empty() {
            let stdout = sanitize_for_json(&stdout_s);
            if !stdout.is_empty() {
                result_parts.push(stdout);
            }
        }

        // Stderr - sanitize to remove control characters and ANSI sequences
        if !output.stderr.is_empty() {
            let stderr = sanitize_for_json(&stderr_s);
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

fn approval_scope_from_params(params: &Value) -> Result<CommandApprovalScope, String> {
    let value = |name: &str| {
        params
            .get(name)
            .and_then(Value::as_str)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    };
    let channel = value("_context_channel")
        .ok_or_else(|| "approval client context is unavailable".to_string())?;
    let chat_id = value("_context_chat_id")
        .ok_or_else(|| "approval client context is unavailable".to_string())?;
    let session_key =
        value("_context_session_key").unwrap_or_else(|| format!("{channel}:{chat_id}"));
    Ok(CommandApprovalScope {
        channel,
        chat_id,
        session_key,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_approval_backend_attaches_guardian_unless_never() {
        let coordinator = CommandApprovalCoordinator::new(std::time::Duration::from_secs(2));
        let trusted =
            ExecTool::new().with_approval_backend(Some(coordinator), AskForApproval::UnlessTrusted);
        assert!(
            trusted.has_guardian_for_test(),
            "trusted mode must attach a Guardian in production wiring"
        );

        let never = ExecTool::new().with_approval_backend(None, AskForApproval::Never);
        assert!(
            !never.has_guardian_for_test(),
            "Never mode must keep the plain orchestrator path"
        );
    }

    #[tokio::test]
    async fn test_exec_simple_command() {
        let tool = ExecTool::new();
        let params = json!({
            "command": "echo hello"
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

    /// Wave C2：模型传入的 `working_dir` 跳出选定 workspace 根目录时，
    /// 返回包含 workspace 约束说明的拒绝消息。
    #[tokio::test]
    async fn working_dir_outside_workspace_root_is_rejected() {
        let workspace = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let tool = ExecTool::with_config(60, Some(workspace.path().to_path_buf()), true);
        let params = json!({
            "command": "echo boundary",
            "working_dir": outside.path().to_str().unwrap()
        });

        let result = tool.execute(params).await.unwrap();
        assert!(
            result.to_lowercase().contains("workspace"),
            "越界 working_dir 应返回包含 workspace 约束说明的拒绝：{result}"
        );
    }

    /// Wave C2：相对路径 working_dir 必须以 workspace 为基准解析，
    /// 且 `..` 穿越到根外部时同样被拒绝。
    #[tokio::test]
    async fn working_dir_relative_escape_is_rejected() {
        let workspace = tempfile::tempdir().unwrap();
        let tool = ExecTool::with_config(60, Some(workspace.path().to_path_buf()), true);
        let params = json!({
            "command": "echo escape",
            "working_dir": "../../../.."
        });
        let result = tool.execute(params).await.unwrap();
        assert!(
            result.to_lowercase().contains("workspace"),
            "`..` 穿越越界应被拒绝：{result}"
        );
    }

    /// Wave C2：合法路径（workspace 内子目录）正常执行。
    #[tokio::test]
    async fn working_dir_inside_workspace_subdir_runs() {
        let workspace = tempfile::tempdir().unwrap();
        let inner = workspace.path().join("sub-project");
        std::fs::create_dir_all(&inner).unwrap();
        let tool = ExecTool::with_config(60, Some(workspace.path().to_path_buf()), true);
        let params = json!({
            "command": if cfg!(target_os = "windows") { "echo hello" } else { "echo hello" },
            "working_dir": inner.to_str().unwrap()
        });
        let result = tool.execute(params).await.unwrap();
        assert!(
            result.contains("hello"),
            "workspace 内子目录应正常执行：{result}"
        );
    }

    #[tokio::test]
    async fn test_exec_timeout() {
        let tool = ExecTool::with_config(1, None, false);
        let params = json!({
            "command": if cfg!(target_os = "windows") {
                // Use Start-Sleep for PowerShell
                "Start-Sleep -Seconds 6"
            } else {
                "sleep 5"
            }
        });

        let result = tool.execute(params).await.unwrap();
        assert!(result.contains("timed out"));
    }

    #[tokio::test]
    async fn approval_resumes_the_same_exec_call_once() {
        let coordinator = CommandApprovalCoordinator::new(std::time::Duration::from_secs(2));
        let mut config = SandboxConfig::danger_full_access();
        config.mode = agent_diva_sandbox::SandboxMode::WorkspaceWrite;
        config.approval_policy = agent_diva_sandbox::AskForApproval::OnRequest;
        config.writable_roots = vec![std::env::current_dir().unwrap()];
        let manager = Arc::new(SandboxManager::new(&config));
        let orchestrator = Arc::new(ToolOrchestrator::new(
            manager,
            agent_diva_sandbox::AskForApproval::OnRequest,
        ));
        let tool = ExecTool::new().with_orchestrator(orchestrator, Some(coordinator.clone()));
        let task = tokio::spawn(async move {
            tool.execute(json!({
                "command": "echo approval-resumed",
                "_context_channel": "api",
                "_context_chat_id": "chat",
                "_context_session_key": "api:chat"
            }))
            .await
        });
        tokio::task::yield_now().await;
        let pending = coordinator.pending(None).await;
        assert_eq!(pending.len(), 1);
        coordinator
            .resolve(
                &pending[0].approval_id,
                agent_diva_sandbox::ApprovalDecision::ApproveOnce,
            )
            .await
            .unwrap();
        let output = task.await.unwrap().unwrap();
        assert!(output.contains("approval-resumed"));
        assert!(coordinator.pending(None).await.is_empty());
    }

    #[tokio::test]
    async fn global_rule_reuses_a_real_command_across_sessions() {
        let dir = tempfile::tempdir().unwrap();
        let rules = Arc::new(
            agent_diva_sandbox::CommandRuleStore::open(dir.path().join("execpolicy.toml")).unwrap(),
        );
        let coordinator = CommandApprovalCoordinator::new(std::time::Duration::from_secs(2))
            .with_command_rules(rules.clone());
        let mut config = SandboxConfig::danger_full_access();
        config.mode = agent_diva_sandbox::SandboxMode::WorkspaceWrite;
        config.approval_policy = agent_diva_sandbox::AskForApproval::OnRequest;
        config.writable_roots = vec![std::env::current_dir().unwrap()];
        let manager = Arc::new(SandboxManager::new(&config));
        let orchestrator = Arc::new(ToolOrchestrator::new(
            manager,
            agent_diva_sandbox::AskForApproval::OnRequest,
        ));
        let first_tool =
            ExecTool::new().with_orchestrator(orchestrator.clone(), Some(coordinator.clone()));
        let first = tokio::spawn(async move {
            first_tool
                .execute(json!({
                    "command": "git --version",
                    "_context_channel": "api",
                    "_context_chat_id": "first",
                    "_context_session_key": "api:first"
                }))
                .await
        });
        tokio::task::yield_now().await;
        let pending = coordinator.pending(None).await;
        coordinator
            .resolve(
                &pending[0].approval_id,
                agent_diva_sandbox::ApprovalDecision::ApproveGlobal,
            )
            .await
            .unwrap();
        assert!(first.await.unwrap().unwrap().contains("git version"));

        let second_tool =
            ExecTool::new().with_orchestrator(orchestrator, Some(coordinator.clone()));
        let second = second_tool
            .execute(json!({
                "command": "git --version",
                "_context_channel": "api",
                "_context_chat_id": "second",
                "_context_session_key": "api:second"
            }))
            .await
            .unwrap();
        assert!(second.contains("git version"));
        assert!(coordinator.pending(None).await.is_empty());
        assert_eq!(rules.list().len(), 1);
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

    #[test]
    fn test_sanitize_output_removes_ansi_colors() {
        // Red text: ESC[31mhello ESC[0m
        let input = "\x1b[31mhello\x1b[0m world";
        let output = sanitize_for_json(input);
        assert_eq!(output, "hello world");
    }

    #[test]
    fn test_sanitize_output_removes_ansi_cursor_movement() {
        // ESC[2J clears screen, ESC[H moves cursor home
        let input = "\x1b[2J\x1b[Hcontent";
        let output = sanitize_for_json(input);
        assert_eq!(output, "content");
    }

    #[test]
    fn test_sanitize_output_removes_control_chars() {
        // Include NULL, BEL, and other control characters
        let input = "hello\x00world\x07bell\x01\x02";
        let output = sanitize_for_json(input);
        assert_eq!(output, "helloworldbell");
    }

    #[test]
    fn test_sanitize_output_preserves_whitespace() {
        // Tab, newline, carriage return should be preserved
        let input = "line1\nline2\r\nline3\tindented";
        let output = sanitize_for_json(input);
        assert_eq!(output, "line1\nline2\r\nline3\tindented");
    }

    #[test]
    fn test_sanitize_output_preserves_unicode() {
        // Chinese, emoji, etc. should be preserved
        let input = "你好世界 🐈 日本語";
        let output = sanitize_for_json(input);
        assert_eq!(output, "你好世界 🐈 日本語");
    }

    #[test]
    fn test_sanitize_output_complex_ansi() {
        // Complex ANSI: bold + color + background
        let input = "\x1b[1;34;47mcolored\x1b[0m text";
        let output = sanitize_for_json(input);
        assert_eq!(output, "colored text");
    }

    /// PowerShell on zh-CN Windows often writes GB18030 to stderr pipes; decoding as UTF-8 yields U+FFFD.
    #[cfg(windows)]
    #[test]
    fn decode_shell_pipe_bytes_handles_gb18030_powershell_stderr() {
        let text = "所在位置 行:1 字符: 10\r\n+ cd foo && bar";
        let (bytes, _, _) = encoding_rs::GB18030.encode(text);
        let decoded = super::decode_shell_pipe_bytes(bytes.as_ref());
        assert!(
            !decoded.contains('\u{FFFD}'),
            "expected no replacement chars, got: {:?}",
            decoded
        );
        assert_eq!(decoded, text);
    }
}
