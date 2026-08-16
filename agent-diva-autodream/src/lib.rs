//! Manual AutoDream run lifecycle and file-first storage.

mod atomic;
mod curation;
mod error;
mod inputs;
mod layout;
mod metrics;
mod monthly;
mod reflection;
mod reports;
mod rhythm;
mod service;
mod worker;

pub use error::{AutoDreamError, Result};
pub use inputs::{
    AutoDreamCollectedInput, AutoDreamCollectedInputs, AutoDreamInputCollector,
    AutoDreamInputCollectorConfig,
};
pub use layout::{AutoDreamPaths, AutoDreamStorage};
pub use metrics::{AutoDreamMetrics, AutoDreamMetricsSnapshot};
pub(crate) use monthly::{AutoDreamMonthlyReportGenerator, MonthlyReportErrorMarker};
pub use reflection::{
    ReflectionError, ReflectionEvidence, SkillReflectionCandidate, SkillReflectionEngine,
    SkillReflectionIndex, SkillReflectionInput, SkillReflectionOutput,
};
pub use reports::{
    AutoDreamReportWriter, RhythmReportContent, RhythmReportPeriod, RhythmReportWriteRequest,
    RhythmReportWriteResult,
};
pub use rhythm::AutoDreamRhythmReportGenerator;
pub use service::{
    AutoDreamCheckpoint, AutoDreamEvent, AutoDreamLockRecord, AutoDreamRunList, AutoDreamRunStatus,
    AutoDreamService, ManualRunTriggerRequest, ScheduledMonthlyReportOutcome,
};
pub use worker::{
    AutoDreamReflectionStage, AutoDreamReflectionStageRecord, AutoDreamRestrictedAction,
    AutoDreamRestrictedProfile, AutoDreamWorker, AutoDreamWorkerConfig, AutoDreamWorkerOutcome,
    AutoDreamWorkerReport, AutoDreamWorkerStageStatus,
};
