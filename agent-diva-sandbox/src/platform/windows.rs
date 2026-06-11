//! Windows sandbox implementation using Restricted Token
//!
//! This module implements process isolation on Windows using:
//! - CreateRestrictedToken API with LUA_TOKEN and WRITE_RESTRICTED flags
//!
//! Inspired by OpenAI Codex CLI's windows-sandbox-rs architecture.

use crate::error::{SandboxError, SandboxResult};
use crate::filesystem::FileSystemSandboxPolicy;
use crate::policy::{SandboxPolicy, WindowsSandboxLevel};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tracing::{debug, info};

use windows::Win32::Foundation::*;
use windows::Win32::Security::*;
use windows::Win32::System::Threading::*;

// ============================================================================
// Constants
// ============================================================================

#[allow(dead_code)]
/// Disable max privilege flag
const DISABLE_MAX_PRIVILEGE: u32 = 0x01;
#[allow(dead_code)]
/// LUA (Least-privileged User Account) token flag
const LUA_TOKEN: u32 = 0x04;
#[allow(dead_code)]
/// Write restricted flag
const WRITE_RESTRICTED: u32 = 0x08;

// ============================================================================
// Windows Sandbox Executor
// ============================================================================

/// Windows sandbox executor using Restricted Token
pub struct WindowsSandboxExecutor {
    level: WindowsSandboxLevel,
}

impl WindowsSandboxExecutor {
    /// Create a new Windows sandbox executor
    pub fn new(level: WindowsSandboxLevel) -> Self {
        Self { level }
    }

    /// Check if sandbox is available and properly configured
    pub fn is_available(&self) -> bool {
        match self.level {
            WindowsSandboxLevel::Disabled => false,
            WindowsSandboxLevel::RestrictedToken => false,
            WindowsSandboxLevel::Elevated => false,
        }
    }

    /// Execute a command in the sandbox
    pub async fn execute(
        &self,
        command: &str,
        cwd: &Path,
        env: HashMap<String, String>,
        timeout_secs: u64,
        policy: &SandboxPolicy,
        fs_policy: &FileSystemSandboxPolicy,
    ) -> SandboxResult<String> {
        match self.level {
            WindowsSandboxLevel::Disabled => {
                self.execute_direct(command, cwd, env, timeout_secs).await
            }
            WindowsSandboxLevel::RestrictedToken => {
                let _ = (command, cwd, env, timeout_secs, policy, fs_policy);
                Err(SandboxError::PlatformError(
                    "Windows restricted-token sandbox is disabled because real restricted-process creation is not implemented".to_string(),
                ))
            }
            WindowsSandboxLevel::Elevated => {
                let _ = (command, cwd, env, timeout_secs, policy, fs_policy);
                Err(SandboxError::PlatformError(
                    "Windows elevated sandbox is not implemented".to_string(),
                ))
            }
        }
    }

    /// Execute directly without sandbox
    pub async fn execute_direct(
        &self,
        command: &str,
        cwd: &Path,
        env: HashMap<String, String>,
        timeout_secs: u64,
    ) -> SandboxResult<String> {
        info!("Executing command directly (no sandbox): {}", command);

        use tokio::process::Command;
        use tokio::time::timeout;

        let mut cmd = Command::new("powershell");
        cmd.arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-Command")
            .arg(command)
            .current_dir(cwd)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());

        for (key, value) in env {
            cmd.env(key, value);
        }

        let child = cmd
            .spawn()
            .map_err(|e| SandboxError::SpawnFailed(e.to_string()))?;

        let output_future = child.wait_with_output();
        let output_result = timeout(Duration::from_secs(timeout_secs), output_future).await;

        let output = match output_result {
            Ok(Ok(out)) => out,
            Ok(Err(e)) => return Err(SandboxError::SpawnFailed(e.to_string())),
            Err(_) => return Err(SandboxError::Timeout { secs: timeout_secs }),
        };

        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if output.status.success() {
            Ok(if stdout.is_empty() { stderr } else { stdout })
        } else {
            let code = output.status.code().unwrap_or(-1);
            Err(SandboxError::ExecutionFailed {
                code,
                stdout,
                stderr,
            })
        }
    }

    /// Create a restricted token with proper restrictions
    #[allow(dead_code)]
    unsafe fn create_restricted_token(&self) -> SandboxResult<HANDLE> {
        debug!("Creating restricted token");

        let current_process = GetCurrentProcess();
        let mut h_token: HANDLE = HANDLE::default();

        let desired_access = TOKEN_DUPLICATE
            | TOKEN_QUERY
            | TOKEN_ASSIGN_PRIMARY
            | TOKEN_ADJUST_DEFAULT
            | TOKEN_ADJUST_PRIVILEGES;

        let result = OpenProcessToken(current_process, desired_access, &mut h_token);
        if result.is_err() {
            let err = GetLastError();
            return Err(SandboxError::TokenCreation(format!(
                "OpenProcessToken failed: {}",
                err.0
            )));
        }

        // Create restricted token with LUA_TOKEN and WRITE_RESTRICTED
        let flags =
            CREATE_RESTRICTED_TOKEN_FLAGS(DISABLE_MAX_PRIVILEGE | LUA_TOKEN | WRITE_RESTRICTED);

        // Output handle for the new token
        let mut new_token_handle: HANDLE = HANDLE::default();

        let result = CreateRestrictedToken(h_token, flags, None, None, None, &mut new_token_handle);

        // Close original token handle
        let _ = CloseHandle(h_token);

        if result.is_ok() {
            debug!("Restricted token created successfully with WRITE_RESTRICTED");
            Ok(new_token_handle)
        } else {
            let err = GetLastError();
            Err(SandboxError::TokenCreation(format!(
                "CreateRestrictedToken failed: {}",
                err.0
            )))
        }
    }
    fn execute_restricted_blocking(
        command: &str,
        cwd: &Path,
        env: HashMap<String, String>,
        timeout_secs: u64,
    ) -> SandboxResult<String> {
        info!(
            "Executing command in Windows restricted sandbox: {}",
            command
        );

        unsafe {
            let token = HandleGuard::new(
                Self::new(WindowsSandboxLevel::RestrictedToken).create_restricted_token()?,
            );
            let job = HandleGuard::new(create_kill_on_close_job()?);
            let stdout_pipe = InheritedPipe::new()?;
            let stderr_pipe = InheritedPipe::new()?;
            let stdin_pipe = InheritedPipe::new()?;

            let mut startup = STARTUPINFOW::default();
            startup.cb = size_of::<STARTUPINFOW>() as u32;
            startup.dwFlags = STARTF_USESTDHANDLES;
            startup.hStdInput = stdin_pipe.read_handle();
            startup.hStdOutput = stdout_pipe.write_handle();
            startup.hStdError = stderr_pipe.write_handle();

            let mut process_info = PROCESS_INFORMATION::default();
            let mut command_line = to_wide_mut(&powershell_command_line(command));
            let cwd_wide = to_wide(cwd.as_os_str());
            let env_block = build_environment_block(env);

            let creation_flags = CREATE_SUSPENDED | CREATE_UNICODE_ENVIRONMENT;
            let create_result = CreateProcessAsUserW(
                token.handle(),
                PCWSTR::null(),
                PWSTR(command_line.as_mut_ptr()),
                None,
                None,
                true,
                creation_flags,
                Some(env_block.as_ptr().cast()),
                PCWSTR(cwd_wide.as_ptr()),
                &startup,
                &mut process_info,
            )
            .or_else(|_| {
                CreateProcessWithTokenW(
                    token.handle(),
                    CREATE_PROCESS_LOGON_FLAGS(0),
                    PCWSTR::null(),
                    PWSTR(command_line.as_mut_ptr()),
                    creation_flags,
                    Some(env_block.as_ptr().cast()),
                    PCWSTR(cwd_wide.as_ptr()),
                    &startup,
                    &mut process_info,
                )
            });

            create_result.map_err(|e| SandboxError::SpawnFailed(e.to_string()))?;

            let process = HandleGuard::new(process_info.hProcess);
            let thread_handle = HandleGuard::new(process_info.hThread);

            AssignProcessToJobObject(job.handle(), process.handle()).map_err(|e| {
                SandboxError::PlatformError(format!("AssignProcessToJobObject failed: {e}"))
            })?;

            if ResumeThread(thread_handle.handle()) == u32::MAX {
                return Err(SandboxError::SpawnFailed(format!(
                    "ResumeThread failed: {}",
                    GetLastError().0
                )));
            }

            drop(thread_handle);
            drop(stdin_pipe);

            let stdout_reader = stdout_pipe.into_reader();
            let stderr_reader = stderr_pipe.into_reader();

            let wait_ms = timeout_secs.saturating_mul(1000).min(u32::MAX as u64) as u32;
            let wait = WaitForSingleObject(process.handle(), wait_ms);

            if wait == WAIT_TIMEOUT {
                let _ = TerminateJobObject(job.handle(), 1);
                let _ = WaitForSingleObject(process.handle(), 5_000);
                let _ = stdout_reader.join();
                let _ = stderr_reader.join();
                return Err(SandboxError::Timeout { secs: timeout_secs });
            }

            if wait != WAIT_OBJECT_0 {
                let _ = TerminateJobObject(job.handle(), 1);
                return Err(SandboxError::PlatformError(format!(
                    "WaitForSingleObject failed: {:?}",
                    wait
                )));
            }

            let stdout = stdout_reader.join()?;
            let stderr = stderr_reader.join()?;

            let mut exit_code = 0;
            GetExitCodeProcess(process.handle(), &mut exit_code).map_err(|e| {
                SandboxError::PlatformError(format!("GetExitCodeProcess failed: {e}"))
            })?;

            let stdout = String::from_utf8_lossy(&stdout).into_owned();
            let stderr = String::from_utf8_lossy(&stderr).into_owned();

            if exit_code == 0 {
                Ok(if stdout.is_empty() { stderr } else { stdout })
            } else {
                Err(SandboxError::ExecutionFailed {
                    code: i32::try_from(exit_code).unwrap_or(-1),
                    stdout,
                    stderr,
                })
            }
        }
    }
}

impl Default for WindowsSandboxExecutor {
    fn default() -> Self {
        Self::new(WindowsSandboxLevel::default())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_creation() {
        let executor = WindowsSandboxExecutor::new(WindowsSandboxLevel::RestrictedToken);
        assert!(!executor.is_available());
    }

    #[test]
    fn test_executor_disabled() {
        let executor = WindowsSandboxExecutor::new(WindowsSandboxLevel::Disabled);
        assert!(!executor.is_available());
    }

    #[test]
    fn test_executor_default() {
        let executor = WindowsSandboxExecutor::default();
        assert!(!executor.is_available());
    }

    #[tokio::test]
    async fn test_direct_execution() {
        let executor = WindowsSandboxExecutor::new(WindowsSandboxLevel::Disabled);
        let result = executor
            .execute_direct("echo hello", Path::new("."), HashMap::new(), 10)
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("hello"));
    }

    #[tokio::test]
    async fn test_direct_execution_with_env() {
        let executor = WindowsSandboxExecutor::new(WindowsSandboxLevel::Disabled);
        let env = HashMap::from([("TEST_VAR".to_string(), "test_value".to_string())]);
        let result = executor
            .execute_direct("echo $env:TEST_VAR", Path::new("."), env, 10)
            .await;

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("test_value"));
    }

    #[test]
    fn test_token_creation() {
        let executor = WindowsSandboxExecutor::new(WindowsSandboxLevel::RestrictedToken);

        unsafe {
            if let Ok(token) = executor.create_restricted_token() {
                let _ = CloseHandle(token);
            }
        }
    }

    #[tokio::test]
    async fn test_restricted_token_execution() {
        let executor = WindowsSandboxExecutor::new(WindowsSandboxLevel::RestrictedToken);
        let policy = SandboxPolicy::default();
        let fs_policy = FileSystemSandboxPolicy::unrestricted();

        let result = executor
            .execute(
                "echo hello",
                Path::new("."),
                HashMap::new(),
                30,
                &policy,
                &fs_policy,
            )
            .await;

        assert!(matches!(result, Err(SandboxError::PlatformError(_))));
    }
}
