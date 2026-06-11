//! Windows sandbox implementation using Restricted Token + Job Object
//!
//! This module implements process isolation on Windows using:
//! - CreateRestrictedToken API with LUA_TOKEN privilege reduction
//! - Job Object cleanup so the whole child process tree is killed on timeout/drop
//!
//! Inspired by OpenAI Codex CLI's windows-sandbox-rs architecture.

use crate::error::{SandboxError, SandboxResult};
use crate::filesystem::FileSystemSandboxPolicy;
use crate::policy::{SandboxPolicy, WindowsSandboxLevel};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::mem::size_of;
use std::os::windows::ffi::OsStrExt;
use std::path::Path;
use std::thread;
use std::time::Duration;
use tracing::{debug, info};
use windows::core::{PCWSTR, PWSTR};

use windows::Win32::Foundation::*;
use windows::Win32::Security::*;
use windows::Win32::Storage::FileSystem::ReadFile;
use windows::Win32::System::JobObjects::*;
use windows::Win32::System::Pipes::CreatePipe;
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
#[allow(dead_code)]
/// Write restricted flag. Kept for future hardening; enabling it breaks common
/// Windows shell initialization paths in the current minimum viable sandbox.
const WRITE_RESTRICTED: u32 = 0x08;

const PIPE_READ_BUFFER_SIZE: usize = 16 * 1024;

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
            WindowsSandboxLevel::RestrictedToken | WindowsSandboxLevel::Elevated => unsafe {
                match self.create_restricted_token() {
                    Ok(token) => {
                        let _ = CloseHandle(token);
                        true
                    }
                    Err(err) => {
                        debug!("Windows sandbox unavailable: {}", err);
                        false
                    }
                }
            },
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
            WindowsSandboxLevel::RestrictedToken | WindowsSandboxLevel::Elevated => {
                self.ensure_command_allowed(command, cwd, policy, fs_policy)?;
                let command = command.to_string();
                let cwd = cwd.to_path_buf();
                tokio::task::spawn_blocking(move || {
                    Self::execute_restricted_blocking(&command, &cwd, env, timeout_secs)
                })
                .await
                .map_err(|e| SandboxError::Internal(format!("Windows sandbox task failed: {e}")))?
            }
        }
    }

    fn ensure_command_allowed(
        &self,
        command: &str,
        cwd: &Path,
        _policy: &SandboxPolicy,
        _fs_policy: &FileSystemSandboxPolicy,
    ) -> SandboxResult<()> {
        if command.trim().is_empty() {
            return Err(SandboxError::InvalidCommand(
                "command cannot be empty".to_string(),
            ));
        }

        if !cwd.exists() {
            return Err(SandboxError::InvalidCommand(format!(
                "working directory does not exist: {}",
                cwd.display()
            )));
        }

        Ok(())
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

        // Create a LUA token for the minimum viable sandbox. Job Object process
        // tree control below provides the hard boundary for child cleanup.
        let flags = CREATE_RESTRICTED_TOKEN_FLAGS(DISABLE_MAX_PRIVILEGE | LUA_TOKEN);

        // Output handle for the new token
        let mut new_token_handle: HANDLE = HANDLE::default();

        let result = CreateRestrictedToken(h_token, flags, None, None, None, &mut new_token_handle);

        // Close original token handle
        let _ = CloseHandle(h_token);

        if result.is_ok() {
            debug!("Restricted token created successfully");
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
        assert!(executor.is_available());
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

        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.contains("hello"), "output was: {output}");
    }
}

struct HandleGuard(HANDLE);

impl HandleGuard {
    fn new(handle: HANDLE) -> Self {
        Self(handle)
    }

    fn handle(&self) -> HANDLE {
        self.0
    }

    fn into_handle(mut self) -> HANDLE {
        let handle = self.0;
        self.0 = HANDLE::default();
        handle
    }
}

impl Drop for HandleGuard {
    fn drop(&mut self) {
        if !self.0.is_invalid() {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

struct InheritedPipe {
    read: Option<HandleGuard>,
    write: Option<HandleGuard>,
}

impl InheritedPipe {
    unsafe fn new() -> SandboxResult<Self> {
        let security_attributes = SECURITY_ATTRIBUTES {
            nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: std::ptr::null_mut(),
            bInheritHandle: TRUE,
        };
        let mut read = HANDLE::default();
        let mut write = HANDLE::default();

        CreatePipe(&mut read, &mut write, Some(&security_attributes), 0)
            .map_err(|e| SandboxError::SpawnFailed(format!("CreatePipe failed: {e}")))?;
        SetHandleInformation(read, HANDLE_FLAG_INHERIT.0, HANDLE_FLAGS(0))
            .map_err(|e| SandboxError::SpawnFailed(format!("SetHandleInformation failed: {e}")))?;

        Ok(Self {
            read: Some(HandleGuard::new(read)),
            write: Some(HandleGuard::new(write)),
        })
    }

    fn read_handle(&self) -> HANDLE {
        self.read
            .as_ref()
            .map(HandleGuard::handle)
            .unwrap_or_default()
    }

    fn write_handle(&self) -> HANDLE {
        self.write
            .as_ref()
            .map(HandleGuard::handle)
            .unwrap_or_default()
    }

    fn into_reader(mut self) -> PipeReader {
        drop(self.write.take());
        let handle = self
            .read
            .take()
            .expect("pipe read handle missing")
            .into_handle();
        PipeReader::spawn(handle)
    }
}

struct PipeReader {
    join_handle: thread::JoinHandle<SandboxResult<Vec<u8>>>,
}

impl PipeReader {
    fn spawn(handle: HANDLE) -> Self {
        let raw_handle = handle.0 as isize;
        let join_handle = thread::spawn(move || {
            let handle = HANDLE(raw_handle as _);
            unsafe { read_pipe_to_end(handle) }
        });
        Self { join_handle }
    }

    fn join(self) -> SandboxResult<Vec<u8>> {
        self.join_handle
            .join()
            .map_err(|_| SandboxError::Internal("pipe reader thread panicked".to_string()))?
    }
}

unsafe fn read_pipe_to_end(handle: HANDLE) -> SandboxResult<Vec<u8>> {
    let guard = HandleGuard::new(handle);
    let mut output = Vec::new();
    loop {
        let mut buffer = [0u8; PIPE_READ_BUFFER_SIZE];
        let mut bytes_read = 0;
        match ReadFile(
            guard.handle(),
            Some(&mut buffer),
            Some(&mut bytes_read),
            None,
        ) {
            Ok(()) => {
                if bytes_read == 0 {
                    break;
                }
                output.extend_from_slice(&buffer[..bytes_read as usize]);
            }
            Err(_) => break,
        }
    }
    Ok(output)
}

unsafe fn create_kill_on_close_job() -> SandboxResult<HANDLE> {
    let job = CreateJobObjectW(None, PCWSTR::null())
        .map_err(|e| SandboxError::PlatformError(format!("CreateJobObjectW failed: {e}")))?;

    let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

    SetInformationJobObject(
        job,
        JobObjectExtendedLimitInformation,
        (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
        size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
    )
    .map_err(|e| {
        let _ = CloseHandle(job);
        SandboxError::PlatformError(format!("SetInformationJobObject failed: {e}"))
    })?;

    Ok(job)
}

fn powershell_command_line(command: &str) -> String {
    format!(
        "powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command {}",
        quote_windows_arg(command)
    )
}

fn quote_windows_arg(arg: &str) -> String {
    let escaped = arg.replace('"', "\\\"");
    format!("\"{}\"", escaped)
}

fn to_wide(value: &OsStr) -> Vec<u16> {
    value.encode_wide().chain(std::iter::once(0)).collect()
}

fn to_wide_mut(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

fn build_environment_block(extra_env: HashMap<String, String>) -> Vec<u16> {
    let mut env: Vec<(String, String)> = std::env::vars().collect();
    env.extend(extra_env);
    env.sort_by(|a, b| a.0.to_uppercase().cmp(&b.0.to_uppercase()));

    let mut block = Vec::new();
    for (key, value) in env {
        if key.contains('=') {
            continue;
        }
        block.extend(OsStr::new(&format!("{key}={value}")).encode_wide());
        block.push(0);
    }
    block.push(0);
    block
}
