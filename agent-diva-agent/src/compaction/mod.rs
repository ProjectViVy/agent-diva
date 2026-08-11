//! Canonical checkpoint compaction — bounded, replacement-based context state.
//!
//! This module provides token estimation, budget monitoring, and LLM-driven
//! compaction that keeps long-running agent sessions within the provider's
//! context window.
//!
//! # Architecture (P0)
//!
//! ```text
//! TokenEstimator ──► ContextBudgetMonitor ──► CheckpointCompactor
//!      │                      │                       │
//!  chars→tokens          .check()               .compact()
//!                         budget→pct             history→summary
//! ```
//!
//! See ADR-0010 for the full design.
//!
//! # Note
//!
//! `token_estimate` and `context_budget` live at the crate root
//! (`agent_diva_agent::token_estimate`, `agent_diva_agent::context_budget`)
//! and are re-used by this module.

pub mod compaction_exec;
pub mod prompt;
pub mod quality;

pub use compaction_exec::{
    select_safe_compaction_end, CheckpointCompactor, CheckpointSnapshot, PendingCheckpointUpdate,
};
pub use prompt::{CHECKPOINT_PROMPT_ID, CHECKPOINT_PROMPT_VERSION, CHECKPOINT_SYSTEM_PROMPT};
pub use quality::{validate_summary, QualityGate, QualityReport, QualityResult};
