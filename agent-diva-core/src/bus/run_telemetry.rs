//! FR22 / ADR-E：运行遥测快照（白名单字段、版本化）。**不**写入用户 transcript（NFR-R2）。
//!
//! # 语义（Story 5.2 单一权威）
//!
//! - **`internal_step_count`：** 本 turn 内 **主 ReAct 循环** 迭代次数（`AgentLoop` / `max_iterations`），**不含**蜂群序曲中的 LLM 调用。
//! - **`prelude_llm_calls`：** FullSwarm 且启用过程管道时，序曲（`run_swarm_deliberation_prelude`）内 **成功完成** 的 LLM `chat` 次数；否则 `0`。
//! - **`phase_count`：** 与过程管道中 **`swarm_phase_changed` 发射次数** 对齐：序曲内计数 + 主循环内每次迭代至多 1 条（`agent_iteration_k`）；无管道时不累计主循环相位。
//! - **`full_swarm_convergence_rounds`：** FullSwarm turn 末尾 `execute_full_swarm_convergence_loop` 返回的 `rounds_completed`；无收敛阶段为 `None`。
//!
//! Headless 桩映射见 `agent_diva_swarm::run_telemetry_from_minimal_turn_trace`。

use serde::{Deserialize, Serialize};

/// 线协议 schema 版本（v0 = 0, v1 = 1）；演进时 bump 并与 GUI 对齐（NFR-I2）。
pub const RUN_TELEMETRY_SCHEMA_VERSION_V0: u32 = 0;
pub const RUN_TELEMETRY_SCHEMA_VERSION_V1: u32 = 1;

/// 单次用户 turn 完成后对开发者可见的内部用量摘要（与 `architecture.md` ADR-E 对齐）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunTelemetrySnapshotV0 {
    pub schema_version: u32,
    /// 主 ReAct 循环迭代次数（**不含**序曲 LLM）。
    pub internal_step_count: u32,
    /// 蜂群序曲 LLM 调用次数（非 FullSwarm / 无管道 / 序曲未启用为 `0`）。
    #[serde(default)]
    pub prelude_llm_calls: u32,
    /// 与 `swarm_phase_changed` **try_emit** 次数对齐：序曲 + 主循环（有管道时每迭代 1 次）。
    pub phase_count: u32,
    /// `Some(true)` 表示本 turn **触顶** `max_iterations` 仍未产出终局响应（超建议预算，琥珀提示）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub over_suggested_budget: Option<bool>,
    /// FullSwarm 收敛循环的 `rounds_completed`；未进入收敛为 `None`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_swarm_convergence_rounds: Option<u32>,
}

impl RunTelemetrySnapshotV0 {
    /// 网关 / AgentLoop 实时路径构造（SSE `run_telemetry` 与 GUI 缓存同源）。
    #[must_use]
    pub fn from_live_agent_turn(
        prelude_llm_calls: u32,
        prelude_swarm_phase_events: u32,
        main_loop_iterations: u32,
        main_loop_swarm_phase_events: u32,
        hit_iteration_cap_without_final: bool,
        full_swarm_convergence_rounds: Option<u32>,
    ) -> Self {
        Self {
            schema_version: RUN_TELEMETRY_SCHEMA_VERSION_V1,
            internal_step_count: main_loop_iterations,
            prelude_llm_calls,
            phase_count: prelude_swarm_phase_events.saturating_add(main_loop_swarm_phase_events),
            over_suggested_budget: Some(hit_iteration_cap_without_final),
            full_swarm_convergence_rounds,
        }
    }

    /// 兼容旧语义：无蜂群序曲、主循环相位 tick 数等于迭代次数（测试 / 迁移用）。
    #[must_use]
    pub fn from_agent_loop_iteration(
        iterations_executed: u32,
        hit_iteration_cap_without_final: bool,
    ) -> Self {
        Self::from_live_agent_turn(
            0,
            0,
            iterations_executed,
            iterations_executed,
            hit_iteration_cap_without_final,
            None,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_roundtrip_camel_case_v1() {
        let s = RunTelemetrySnapshotV0 {
            schema_version: 1,
            internal_step_count: 3,
            prelude_llm_calls: 2,
            phase_count: 5,
            over_suggested_budget: Some(false),
            full_swarm_convergence_rounds: Some(0),
        };
        let j = serde_json::to_string(&s).unwrap();
        assert!(j.contains("schemaVersion"));
        assert!(j.contains("internalStepCount"));
        assert!(j.contains("preludeLlmCalls"));
        assert!(j.contains("fullSwarmConvergenceRounds"));
        let back: RunTelemetrySnapshotV0 = serde_json::from_str(&j).unwrap();
        assert_eq!(back, s);
    }

    #[test]
    fn serde_deserialize_v0_omitted_prelude_defaults() {
        let j = r#"{"schemaVersion":0,"internalStepCount":2,"phaseCount":2,"overSuggestedBudget":false}"#;
        let back: RunTelemetrySnapshotV0 = serde_json::from_str(j).unwrap();
        assert_eq!(back.prelude_llm_calls, 0);
        assert_eq!(back.full_swarm_convergence_rounds, None);
    }

    #[test]
    fn from_agent_loop_iteration_sets_v1_schema_and_zero_prelude() {
        let s = RunTelemetrySnapshotV0::from_agent_loop_iteration(4, true);
        assert_eq!(s.schema_version, RUN_TELEMETRY_SCHEMA_VERSION_V1);
        assert_eq!(s.internal_step_count, 4);
        assert_eq!(s.prelude_llm_calls, 0);
        assert_eq!(s.phase_count, 4);
        assert_eq!(s.over_suggested_budget, Some(true));
        assert_eq!(s.full_swarm_convergence_rounds, None);
    }
}
