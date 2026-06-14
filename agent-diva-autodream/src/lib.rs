//! Manual AutoDream run lifecycle and file-first storage.

mod atomic;
mod error;
mod inputs;
mod layout;
mod outputs;
mod service;

pub use error::{AutoDreamError, Result};
pub use inputs::{
    AutoDreamCollectedInput, AutoDreamCollectedInputs, AutoDreamInputCollector,
    AutoDreamInputCollectorConfig,
};
pub use layout::{AutoDreamPaths, AutoDreamStorage};
pub use outputs::{
    AutoDreamArtifactSummary, AutoDreamOutputEmitter, AutoDreamOutputEvent,
    AutoDreamOutputEventKind, AutoDreamOutputRequest, AutoDreamProposalCandidateDraft,
    AutoDreamRunArtifact, EmitOutputsResult, EmittedProposalCandidate,
};
pub use service::{
    AutoDreamCheckpoint, AutoDreamEvent, AutoDreamLockRecord, AutoDreamRunList, AutoDreamRunStatus,
    AutoDreamService, ManualRunTriggerRequest,
};
