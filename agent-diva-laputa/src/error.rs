use std::path::PathBuf;

use crate::proposals::ApplyFailurePoint;
use agent_diva_core::evolution::{ChangelogAction, LaputaSectionName, ProposalState, ProposalType};
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

    #[error("proposal not found: {id}")]
    ProposalNotFound { id: String },

    #[error("proposal already exists: {id}")]
    ProposalAlreadyExists { id: String },

    #[error("changelog record not found: {id}")]
    ChangelogNotFound { id: String },

    #[error("unknown Laputa section: {name}")]
    UnknownSection { name: String },

    #[error("unknown Laputa layer: {name}")]
    UnknownLayer { name: String },

    #[error("invalid proposal {id}: {reason}")]
    InvalidProposal { id: String, reason: String },

    #[error("invalid proposal transition: {from:?} -> {to:?}")]
    InvalidProposalTransition {
        from: ProposalState,
        to: ProposalState,
    },

    #[error(
        "unauthorized target for proposal {id}: {proposal_type:?} cannot write {target_section:?}"
    )]
    UnauthorizedTarget {
        id: String,
        proposal_type: ProposalType,
        target_section: LaputaSectionName,
    },

    #[error("schema mismatch for proposal {id}: {reason}")]
    SchemaMismatch { id: String, reason: String },

    #[error("schema incompatible for proposal {id}: {reason}")]
    SchemaIncompatible { id: String, reason: String },

    #[error("unresolved conflict for proposal {id}: {reason}")]
    UnresolvedConflict { id: String, reason: String },

    #[error("rollback expired for changelog {id}")]
    RollbackExpired { id: String },

    #[error("changelog {id} cannot be rolled back because action {action:?} is not eligible")]
    RollbackIneligible { id: String, action: ChangelogAction },

    #[error("rollback conflict for changelog {id}: {reason}")]
    RollbackConflict { id: String, reason: String },

    #[error("rollback failed for proposal {id}: {source}")]
    RollbackFailed {
        id: String,
        #[source]
        source: Box<LaputaError>,
    },

    #[error("injected apply failure at {point:?}")]
    InjectedApplyFailure { point: ApplyFailurePoint },

    #[error("injected migration failure at {point:?}")]
    InjectedMigrationFailure {
        point: crate::migration::LaputaMigrationTestFailure,
    },
}

impl LaputaError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Stable API code used by HTTP, Tauri, diagnostics, and logs.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io_error",
            Self::Json(_) => "json_error",
            Self::LockTimeout { .. } => "lock_timeout",
            Self::ProposalNotFound { .. } | Self::ChangelogNotFound { .. } => "not_found",
            Self::ProposalAlreadyExists { .. } => "proposal_already_exists",
            Self::UnknownSection { .. } => "unknown_section",
            Self::UnknownLayer { .. } => "unknown_layer",
            Self::InvalidProposal { .. } => "invalid_proposal",
            Self::InvalidProposalTransition { .. } => "invalid_proposal_transition",
            Self::UnauthorizedTarget { .. } => "unauthorized_target",
            Self::SchemaMismatch { .. } | Self::SchemaIncompatible { .. } => "schema_incompatible",
            Self::UnresolvedConflict { .. } => "conflict_unresolved",
            Self::RollbackExpired { .. } => "rollback_expired",
            Self::RollbackIneligible { .. } => "rollback_ineligible",
            Self::RollbackConflict { .. } => "rollback_conflict",
            Self::RollbackFailed { .. } => "rollback_failed",
            Self::InjectedApplyFailure { .. } => "apply_recovery_failure",
            Self::InjectedMigrationFailure { .. } => "migration_recovery_failure",
        }
    }
}
