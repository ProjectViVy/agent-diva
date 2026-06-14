use std::path::PathBuf;

use agent_diva_core::evolution::AutoDreamRunState;
use agent_diva_laputa::LaputaError;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, AutoDreamError>;

#[derive(Debug, Error)]
pub enum AutoDreamError {
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("run not found: {id}")]
    RunNotFound { id: String },

    #[error("active run already exists: {id}")]
    ActiveRunExists { id: String },

    #[error("run {id} is not cancellable from state {state:?}")]
    RunNotCancellable {
        id: String,
        state: AutoDreamRunState,
    },

    #[error("invalid AutoDream state: {0}")]
    InvalidState(String),

    #[error("input collection failed: {0}")]
    InputCollection(String),

    #[error("proposal persistence failed: {0}")]
    ProposalPersistence(#[from] LaputaError),
}

impl AutoDreamError {
    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}
