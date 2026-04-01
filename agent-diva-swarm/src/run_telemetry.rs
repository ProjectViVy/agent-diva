//! FR22 / ADR-E：运行遥测 DTO 与 headless 轨迹映射（与 `architecture.md` 白名单字段一致）。
//!
//! # Headless 与实时 AgentLoop 的字段对齐（Story 5.2）
//!
//! 最小 turn **无**主 ReAct 循环与序曲：`internal_step_count` 固定为 `0`；全蜂群桩的收敛轮次放在
//! `full_swarm_convergence_rounds`。`phase_count` 仍用 `process_events_emitted`（终局 `swarm_run_*` 槽位语义，见
//! [`crate::minimal_turn::MinimalTurnTrace::process_events_emitted`]）。

use agent_diva_core::bus::RunTelemetrySnapshotV0;

use crate::{MinimalTurnTrace, SwarmRunStopReason};

/// 由 [`MinimalTurnTrace`] 推导遥测摘要（Epic 1 桩 / 契约测试用；网关真实 turn 使用 `from_live_agent_turn`）。
#[must_use]
pub fn run_telemetry_from_minimal_turn_trace(trace: &MinimalTurnTrace) -> RunTelemetrySnapshotV0 {
    let conv = trace.full_swarm_internal_rounds;
    let phase = trace.process_events_emitted;
    let over = trace.swarm_stop_reason.map(|r| r == SwarmRunStopReason::BudgetExceeded);
    let convergence = if matches!(
        trace.layer,
        crate::CortexExecutionLayer::FullSwarmOrchestration
    ) {
        Some(conv)
    } else {
        None
    };
    RunTelemetrySnapshotV0 {
        schema_version: agent_diva_core::bus::RUN_TELEMETRY_SCHEMA_VERSION_V1,
        internal_step_count: 0,
        prelude_llm_calls: 0,
        phase_count: phase,
        over_suggested_budget: over,
        full_swarm_convergence_rounds: convergence,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CortexExecutionLayer, MinimalTurnTrace};

    #[test]
    fn budget_exceeded_sets_over_flag() {
        let trace = MinimalTurnTrace {
            layer: CortexExecutionLayer::FullSwarmOrchestration,
            process_events_emitted: 1,
            entered_multi_agent_handoff: true,
            swarm_stop_reason: Some(SwarmRunStopReason::BudgetExceeded),
            full_swarm_internal_rounds: 5,
            explicit_full_swarm_suppressed_by_cortex_off: false,
        };
        let s = run_telemetry_from_minimal_turn_trace(&trace);
        assert_eq!(s.internal_step_count, 0);
        assert_eq!(s.prelude_llm_calls, 0);
        assert_eq!(s.phase_count, 1);
        assert_eq!(s.full_swarm_convergence_rounds, Some(5));
        assert_eq!(s.over_suggested_budget, Some(true));
    }
}
