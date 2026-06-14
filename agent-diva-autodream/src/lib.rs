//! Manual AutoDream run lifecycle and file-first storage.

mod atomic;
mod error;
mod layout;
mod service;

pub use error::{AutoDreamError, Result};
pub use layout::{AutoDreamPaths, AutoDreamStorage};
pub use service::{
    AutoDreamCheckpoint, AutoDreamEvent, AutoDreamLockRecord, AutoDreamRunList, AutoDreamRunStatus,
    AutoDreamService, ManualRunTriggerRequest,
};
