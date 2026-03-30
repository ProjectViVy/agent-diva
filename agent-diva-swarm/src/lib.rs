//! Swarm orchestration crate for agent-diva.
//!
//! **Scope (this crate):** multi-agent / handoff orchestration boundaries; **大脑皮层状态**
//! 的 Rust 权威模型见 [`cortex`]（进程内内存、FR14 单一真相源 — 详见该模块文档）。Gateway 同步与
//! Tauri 契约在 Epic 1 后续故事中补齐。
//!
//! **Dependencies:** uses [`agent_diva_core`] for shared types and errors. Per **ADR-A**,
//! this crate **must not** depend on `agent-diva-meta` (Meta is composed only at
//! runtime / gateway layers).

mod cortex;
mod execution_tier;
mod light_intent_rules;
mod light_path_limits;
mod minimal_turn;
mod process_events;

use serde::{Deserialize, Serialize};

pub use agent_diva_core::Result as CoreResult;
pub use cortex::{
    CortexRuntime, CortexState, CortexSyncDto, CORTEX_DEFAULT_ENABLED,
    CORTEX_STATE_SCHEMA_VERSION_V0,
};
pub use execution_tier::{
    resolve_execution_tier, would_enter_full_swarm_topology, ExecutionTier,
};
pub use light_intent_rules::{
    is_explicit_skill_style_input, is_light_intent, is_short_qa, SHORT_QA_MAX_SCALARS,
};
pub use light_path_limits::{
    LightPathStopReason, LIGHT_PATH_MAX_INTERNAL_STEPS, LIGHT_PATH_MAX_WALL_MS,
};
pub use minimal_turn::{
    run_minimal_turn_headless, CortexExecutionLayer, MinimalTurnTrace,
};
pub use process_events::{
    recorder_sink, ProcessEventBatchSink, ProcessEventNameV0, ProcessEventPipeline,
    ProcessEventRecorder, ProcessEventThrottleConfig, ProcessEventV0,
    PROCESS_EVENT_SCHEMA_VERSION_V0,
};

/// Crate version string from `CARGO_PKG_VERSION`.
#[must_use]
pub fn crate_version() -> &'static str {
    tracing::trace!("agent-diva-swarm crate_version");
    env!("CARGO_PKG_VERSION")
}

/// Placeholder marker for serde / wire boundaries to be filled by later stories.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SwarmBoundaryMarker;

/// Minimal error type so `thiserror` stays exercised in CI (`clippy -D warnings`).
#[derive(Debug, thiserror::Error)]
pub enum SwarmError {
    /// Reserved for future swarm-specific failures.
    #[error("swarm placeholder error")]
    Placeholder,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_crate_version() {
        assert!(!crate_version().is_empty());
    }

    #[test]
    fn smoke_core_result_ok() {
        let r: CoreResult<()> = Ok(());
        assert!(r.is_ok());
    }
}
