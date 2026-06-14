//! Manual AutoDream run lifecycle and file-first storage.

mod atomic;
mod error;
mod inputs;
mod layout;
mod outputs;
mod reports;
mod service;
mod worker;

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
pub use reports::{
    AutoDreamReportWriter, RhythmReportContent, RhythmReportPeriod, RhythmReportWriteRequest,
    RhythmReportWriteResult,
};
pub use service::{
    AutoDreamCheckpoint, AutoDreamEvent, AutoDreamLockRecord, AutoDreamRunList, AutoDreamRunStatus,
    AutoDreamService, ManualRunTriggerRequest,
};
pub use worker::{
    AutoDreamReflectionStage, AutoDreamReflectionStageRecord, AutoDreamRestrictedAction,
    AutoDreamRestrictedProfile, AutoDreamWorker, AutoDreamWorkerConfig, AutoDreamWorkerOutcome,
    AutoDreamWorkerReport, AutoDreamWorkerStageStatus,
};
