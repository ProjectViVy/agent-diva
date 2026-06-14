use std::path::PathBuf;

use crate::proposals::ApplyFailurePoint;
use agent_diva_core::evolution::{LaputaSectionName, ProposalState, ProposalType};
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

    #[error("unresolved conflict for proposal {id}: {reason}")]
    UnresolvedConflict { id: String, reason: String },

    #[error("rollback failed for proposal {id}: {source}")]
    RollbackFailed {
        id: String,
        #[source]
        source: Box<LaputaError>,
    },

    #[error("injected apply failure at {point:?}")]
    InjectedApplyFailure { point: ApplyFailurePoint },
}

impl LaputaError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
