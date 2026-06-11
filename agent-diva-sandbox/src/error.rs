//! Error types for sandbox operations

use thiserror::Error;

/// Sandbox operation errors
#[derive(Debug, Error)]
pub enum SandboxError {
    /// Command was denied by sandbox policy
    #[error("Command denied by sandbox policy: {reason}")]
    Denied { reason: String },

    /// Execution requires explicit user approval before it can proceed
    #[error("Command requires approval: {reason}")]
    ApprovalRequired { reason: String },

    /// Permission denied for file system access
    #[error("Permission denied for path '{path}': {reason}")]
    PermissionDenied { path: String, reason: String },

    /// Failed to create restricted token (Windows)
    #[cfg(windows)]
    #[error("Failed to create restricted token: {0}")]
    TokenCreation(String),

    /// Failed to spawn sandboxed process
    #[error("Failed to spawn sandboxed process: {0}")]
    SpawnFailed(String),

    /// Command execution timed out
    #[error("Command timed out after {secs} seconds")]
    Timeout { secs: u64 },

    /// Command completed with a non-zero exit code
    #[error("Command failed with exit code {code}")]
    ExecutionFailed {
        code: i32,
        stdout: String,
        stderr: String,
    },

    /// Invalid command or parameters
    #[error("Invalid command: {0}")]
    InvalidCommand(String),

    /// Platform not supported for sandboxing
    #[error("Sandbox not supported on this platform")]
    PlatformNotSupported,

    /// Platform-specific error (e.g., WSL1, missing dependencies)
    #[error("Platform error: {0}")]
    PlatformError(String),

    /// Platform sandbox is unavailable and direct execution must not be used implicitly
    #[error("Sandbox unavailable on {platform}: {reason}")]
    PlatformUnavailable {
        platform: &'static str,
        reason: String,
    },

    /// Sandbox is disabled via environment variable
    #[error("Sandbox disabled by environment variable")]
    Disabled,

    /// Internal error
    #[error("Internal sandbox error: {0}")]
    Internal(String),
}

/// Convenient result type for sandbox operations
pub type SandboxResult<T> = std::result::Result<T, SandboxError>;
