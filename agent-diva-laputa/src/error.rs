use std::path::PathBuf;
use thiserror::Error;

/// Result type used by Laputa storage primitives.
pub type Result<T> = std::result::Result<T, LaputaError>;

/// Errors returned by Laputa file-first storage.
#[derive(Debug, Error)]
pub enum LaputaError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("timed out acquiring lock {path}")]
    LockTimeout { path: PathBuf },

    #[error("typed memory store error: {0}")]
    MemoryStore(#[from] crate::typed_store::TypedMemoryStoreError),

    #[error("invalid migration id: {migration_id}")]
    InvalidMemoryMigrationId { migration_id: String },

    #[error("memory migration conflict: {migration_id}")]
    MemoryMigrationConflict { migration_id: String },

    #[error("injected memory migration failure")]
    InjectedMemoryMigrationFailure,
}

impl LaputaError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Stable machine-readable code for HTTP/API error envelopes.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::Json(_) => "json",
            Self::LockTimeout { .. } => "lock_timeout",
            Self::MemoryStore(_) => "memory_store",
            Self::InvalidMemoryMigrationId { .. } => "invalid_memory_migration_id",
            Self::MemoryMigrationConflict { .. } => "memory_migration_conflict",
            Self::InjectedMemoryMigrationFailure => "injected_memory_migration_failure",
        }
    }
}
