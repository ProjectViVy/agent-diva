//! Scheduler module — bridges cron triggers to the SupervisedRun store
//! and provides workspace isolation with sandbox path validation.

pub mod bridge;
pub mod sandbox;

pub use bridge::{CronBridge, DeadLetterEntry, RetryOutcome};
pub use sandbox::{resolve_sandbox_path, SandboxRoot, WorkspaceRoots};
