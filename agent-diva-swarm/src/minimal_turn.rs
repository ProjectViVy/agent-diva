//! 最小用户 turn 路径（headless，Story 1.4 + FR19 Story 1.7 + **FR20 Story 1.8**）：与 **大脑皮层开/关**、**执行分层** 及 **全蜂群收敛** 对齐的可测桩。
//!
//! 不发起网络请求、不依赖 GUI。语义登记见 `docs/CORTEX_OFF_SIMPLIFIED_MODE.md`。轻量路由规则见 [`crate::light_intent_rules`]。

use crate::convergence::ConvergencePolicy;
use crate::orchestration_port::{
    SwarmOrchestrationInputV0, SwarmOrchestrationPort, DEFAULT_SWARM_ORCHESTRATION_PORT,
};
use crate::process_events::ProcessEventPipeline;
use crate::{resolve_execution_tier, CortexRuntime, ExecutionTier, SwarmRunStopReason};

/// 单次最小 turn 在编排层的归类（开/关路径须可区分，供 headless 断言）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CortexExecutionLayer {
    /// 大脑皮层关：简化路径，不进入全蜂群 handoff。
    Simplified,
    /// 大脑皮层开且 **FR19 轻量路径**：不进入多参与者 handoff 全图。
    LightPath,
    /// 大脑皮层开且 **非轻量**（或显式全蜂群）：全蜂群编排桩（多代理 handoff 占位）。
    FullSwarmOrchestration,
}

/// 可观测轨迹（固定桩，无网络）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimalTurnTrace {
    pub layer: CortexExecutionLayer,
    /// **终局 `swarm_run_*` 槽位计数（已冻结，非「本 turn 全部 try_emit 条数」）。**
    ///
    /// - **`0`：** `Simplified` / `LightPath`，或 `FullSwarm` 走 [`run_minimal_turn_headless`]（**无**管道、不发射 `swarm_run_*`）。
    /// - **`1`：** `FullSwarm` 且走 [`run_minimal_turn_headless_with_full_swarm_events`]，收敛循环经管道发射 **恰好一条** 终局类 `swarm_run_finished` / `swarm_run_capped`（与 `swarm_phase_changed`、工具事件等条数 **无关**）。
    ///
    /// 产品/遥测若要真实事件总量，须读 `ProcessEventRecorder` 或网关事件流，**勿**将本字段当作通用计数器。
    pub process_events_emitted: u32,
    /// 是否进入多代理 handoff 链（OFF 必须为 false）。
    pub entered_multi_agent_handoff: bool,
    /// 全蜂群终局原因（仅 [`CortexExecutionLayer::FullSwarmOrchestration`]）。
    pub swarm_stop_reason: Option<SwarmRunStopReason>,
    /// 终局时已完成的全蜂群内部轮次数（非 FullSwarm 为 `0`）。
    pub full_swarm_internal_rounds: u32,
    /// **`true`** 当且仅当调用方传入 `explicit_full_swarm == true` 但皮层为 **关**：仍走简化路径（FR3），全蜂群请求不生效；网关/UI 可据此提示用户打开大脑皮层。
    pub explicit_full_swarm_suppressed_by_cortex_off: bool,
}

fn trace_simplified(explicit_full_swarm_suppressed_by_cortex_off: bool) -> MinimalTurnTrace {
    MinimalTurnTrace {
        layer: CortexExecutionLayer::Simplified,
        process_events_emitted: 0,
        entered_multi_agent_handoff: false,
        swarm_stop_reason: None,
        full_swarm_internal_rounds: 0,
        explicit_full_swarm_suppressed_by_cortex_off,
    }
}

fn trace_light() -> MinimalTurnTrace {
    MinimalTurnTrace {
        layer: CortexExecutionLayer::LightPath,
        process_events_emitted: 0,
        entered_multi_agent_handoff: false,
        swarm_stop_reason: None,
        full_swarm_internal_rounds: 0,
        explicit_full_swarm_suppressed_by_cortex_off: false,
    }
}

/// 执行与 GUI/网络无关的最小 turn，读取 [`CortexRuntime`] 真相源。
///
/// `explicit_full_swarm` 表示用户/设置 **显式** 选择深度编排；`false` 时按 FR19 与 [`resolve_execution_tier`] 路由。
/// 若皮层为 **关**，**FR3** 优先：始终简化路径，且当 `explicit_full_swarm == true` 时
/// [`MinimalTurnTrace::explicit_full_swarm_suppressed_by_cortex_off`] 为 **`true`**（可观测，非静默忽略）。
///
/// **FR20：** FullSwarm 分支 **无** [`ProcessEventPipeline`] 时仍执行有界收敛循环（默认策略），**不**发射 `swarm_run_*`；
/// 需要可观测终局事件时请用 [`run_minimal_turn_headless_with_full_swarm_events`]（此时 [`MinimalTurnTrace::process_events_emitted`] 为 **`1`**，含义见该字段文档）。
#[must_use]
pub fn run_minimal_turn_headless(
    rt: &CortexRuntime,
    user_text: &str,
    explicit_full_swarm: bool,
) -> MinimalTurnTrace {
    let enabled = rt.snapshot().enabled;
    if !enabled {
        return trace_simplified(explicit_full_swarm);
    }
    match resolve_execution_tier(user_text, true, explicit_full_swarm) {
        ExecutionTier::Light => trace_light(),
        ExecutionTier::FullSwarm => {
            let policy = ConvergencePolicy::default();
            let input = SwarmOrchestrationInputV0::from_turn(user_text, explicit_full_swarm);
            let outcome = DEFAULT_SWARM_ORCHESTRATION_PORT.run_convergence(&input, &policy, None);
            MinimalTurnTrace {
                layer: CortexExecutionLayer::FullSwarmOrchestration,
                process_events_emitted: 0,
                entered_multi_agent_handoff: true,
                swarm_stop_reason: Some(outcome.stop_reason),
                full_swarm_internal_rounds: outcome.rounds_completed,
                explicit_full_swarm_suppressed_by_cortex_off: false,
            }
        }
    }
}

/// 与 [`run_minimal_turn_headless`] 相同路由；皮层开且进入 FullSwarm 时经 `pipeline` 发射 **`swarm_run_finished` / `swarm_run_capped`**（FR20）。
#[must_use]
pub fn run_minimal_turn_headless_with_full_swarm_events(
    rt: &CortexRuntime,
    user_text: &str,
    explicit_full_swarm: bool,
    pipeline: &ProcessEventPipeline,
    policy: &ConvergencePolicy,
) -> MinimalTurnTrace {
    let enabled = rt.snapshot().enabled;
    if !enabled {
        return trace_simplified(explicit_full_swarm);
    }
    match resolve_execution_tier(user_text, true, explicit_full_swarm) {
        ExecutionTier::Light => trace_light(),
        ExecutionTier::FullSwarm => {
            let input = SwarmOrchestrationInputV0::from_turn(user_text, explicit_full_swarm);
            let outcome = DEFAULT_SWARM_ORCHESTRATION_PORT.run_convergence(
                &input,
                policy,
                Some(pipeline),
            );
            pipeline.flush_pending();
            MinimalTurnTrace {
                layer: CortexExecutionLayer::FullSwarmOrchestration,
                process_events_emitted: 1,
                entered_multi_agent_handoff: true,
                swarm_stop_reason: Some(outcome.stop_reason),
                full_swarm_internal_rounds: outcome.rounds_completed,
                explicit_full_swarm_suppressed_by_cortex_off: false,
            }
        }
    }
}

#[cfg(test)]
mod cortex_off_tests {
    use super::*;
    use crate::convergence::ConvergencePolicy;
    use crate::process_events::{ProcessEventPipeline, ProcessEventThrottleConfig};
    use crate::{recorder_sink, CortexRuntime, CortexState};
    use std::sync::Arc;
    use std::time::Duration;

    /// FR21（**合并**选型）：皮层 OFF ≡ 强制轻量等价 — **不** 进入 FullSwarm、**不** 预留多视角 handoff 内部回合。
    /// doc-ref: `docs/ADR_FR21_FORCE_LIGHT_AND_CORTEX_OFF.md` §1、§4（Story **1.9** / AC #3）
    #[test]
    fn fr21_merge_off_path_no_full_swarm_extra_rounds() {
        let rt = CortexRuntime::from_state(CortexState::with_enabled(false));
        let trace = run_minimal_turn_headless(&rt, "explicit force-light equivalent under merge", false);
        assert_eq!(trace.layer, CortexExecutionLayer::Simplified);
        assert!(!trace.entered_multi_agent_handoff);
        assert_eq!(
            trace.full_swarm_internal_rounds, 0,
            "合并语义下关路径不得累计全蜂群内部轮次"
        );
        assert_eq!(trace.process_events_emitted, 0);
    }

    /// doc-ref: agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md §2 多参与者蜂群
    #[test]
    fn cortex_off_minimal_turn_skips_full_swarm_handoff() {
        let rt = CortexRuntime::from_state(CortexState::with_enabled(false));
        let trace = run_minimal_turn_headless(&rt, "hello", false);
        assert_eq!(trace.layer, CortexExecutionLayer::Simplified);
        assert!(!trace.explicit_full_swarm_suppressed_by_cortex_off);
        assert!(
            !trace.entered_multi_agent_handoff,
            "大脑皮层为「关」时不应进入全蜂群 handoff；若本断言失败，请检查大脑皮层开/关分支是否颠倒。entered_multi_agent_handoff={}",
            trace.entered_multi_agent_handoff
        );
    }

    /// doc-ref: agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md §2 过程事件
    #[test]
    fn cortex_off_minimal_turn_emits_no_process_events() {
        let rt = CortexRuntime::from_state(CortexState::with_enabled(false));
        let trace = run_minimal_turn_headless(&rt, "ping", false);
        assert!(!trace.explicit_full_swarm_suppressed_by_cortex_off);
        assert_eq!(
            trace.process_events_emitted, 0,
            "大脑皮层关路径不应发射中间过程事件计数（简化模式）；process_events_emitted={}",
            trace.process_events_emitted
        );
    }

    /// doc-ref: agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md §2 / §4 A3 — 使用 **非轻量** 长文本使皮层开进入 FullSwarm，与关路径区分。
    #[test]
    fn cortex_on_and_off_minimal_turn_observable_layers_differ() {
        let on_rt = CortexRuntime::from_state(CortexState::with_enabled(true));
        let off_rt = CortexRuntime::from_state(CortexState::with_enabled(false));
        let long = "x".repeat(crate::light_intent_rules::SHORT_QA_MAX_SCALARS + 1);
        let on_t = run_minimal_turn_headless(&on_rt, &long, false);
        let off_t = run_minimal_turn_headless(&off_rt, &long, false);
        assert_ne!(
            on_t.layer,
            off_t.layer,
            "大脑皮层「开」与「关」应对应不同执行层；若相同则开/关分支可能未实现"
        );
        assert_eq!(on_t.layer, CortexExecutionLayer::FullSwarmOrchestration);
        assert_eq!(off_t.layer, CortexExecutionLayer::Simplified);
        assert!(!off_t.explicit_full_swarm_suppressed_by_cortex_off);
        assert_eq!(on_t.swarm_stop_reason, Some(SwarmRunStopReason::Done));
    }

    /// 决策冻结：皮层 OFF 时 **不** 因显式全蜂群进入 FullSwarm（FR3）；通过轨迹字段可观测「请求被压制」。
    #[test]
    fn cortex_off_explicit_full_swarm_stays_simplified_and_is_observable() {
        let rt = CortexRuntime::from_state(CortexState::with_enabled(false));
        let trace = run_minimal_turn_headless(&rt, "hi", true);
        assert_eq!(trace.layer, CortexExecutionLayer::Simplified);
        assert!(!trace.entered_multi_agent_handoff);
        assert!(trace.explicit_full_swarm_suppressed_by_cortex_off);
    }

    #[test]
    fn cortex_off_explicit_full_swarm_suppressed_with_pipeline_variant() {
        let cort = Arc::new(CortexRuntime::from_state(CortexState::with_enabled(false)));
        let (rec, sink) = recorder_sink();
        let pipe = ProcessEventPipeline::new(
            Arc::clone(&cort),
            sink,
            ProcessEventThrottleConfig {
                flush_interval: Duration::from_secs(60),
                max_batch: 100,
            },
        );
        let policy = ConvergencePolicy::default();
        let long = "x".repeat(crate::light_intent_rules::SHORT_QA_MAX_SCALARS + 1);
        let trace = run_minimal_turn_headless_with_full_swarm_events(
            &cort, &long, true, &pipe, &policy,
        );
        assert_eq!(trace.layer, CortexExecutionLayer::Simplified);
        assert!(trace.explicit_full_swarm_suppressed_by_cortex_off);
        assert_eq!(rec.recorded_event_count(), 0);
    }

    /// FR19 / 旅程五：皮层 ON + 轻量输入 → 最小 turn **不** 进入多 handoff 全图。
    #[test]
    fn cortex_on_light_intent_minimal_turn_skips_full_swarm_handoff() {
        let rt = CortexRuntime::from_state(CortexState::with_enabled(true));
        let trace = run_minimal_turn_headless(&rt, "什么是 session？", false);
        assert_eq!(trace.layer, CortexExecutionLayer::LightPath);
        assert!(
            !trace.entered_multi_agent_handoff,
            "轻量路径不得进入全图 handoff；entered_multi_agent_handoff={}",
            trace.entered_multi_agent_handoff
        );
    }

    /// AC#3：极低 `max_internal_rounds` + FullSwarm + 管道 → 必现 `BudgetExceeded` 与 `swarm_run_capped`。
    #[test]
    fn full_swarm_cap_observable_via_pipeline_without_gui() {
        let cort = Arc::new(CortexRuntime::from_state(CortexState::with_enabled(true)));
        let (rec, sink) = recorder_sink();
        let pipe = ProcessEventPipeline::new(
            Arc::clone(&cort),
            sink,
            ProcessEventThrottleConfig {
                flush_interval: Duration::from_secs(60),
                max_batch: 100,
            },
        );
        let policy = ConvergencePolicy {
            max_internal_rounds: 0,
            allow_unbounded_internal_rounds: false,
            wall_clock_timeout: None,
        };
        let long = "y".repeat(crate::light_intent_rules::SHORT_QA_MAX_SCALARS + 1);
        let trace = run_minimal_turn_headless_with_full_swarm_events(
            &cort, &long, false, &pipe, &policy,
        );
        assert_eq!(trace.layer, CortexExecutionLayer::FullSwarmOrchestration);
        assert_eq!(trace.swarm_stop_reason, Some(SwarmRunStopReason::BudgetExceeded));
        assert!(rec.snapshot().iter().any(|e| {
            e.name == crate::ProcessEventNameV0::SwarmRunCapped
                && e.stop_reason == Some(SwarmRunStopReason::BudgetExceeded)
        }));
    }

    /// Story 6.1：墙钟超时 → `Timeout` + `swarm_run_finished` 可经管道观测。
    #[test]
    fn full_swarm_wall_clock_timeout_observable_via_pipeline() {
        let cort = Arc::new(CortexRuntime::from_state(CortexState::with_enabled(true)));
        let (rec, sink) = recorder_sink();
        let pipe = ProcessEventPipeline::new(
            Arc::clone(&cort),
            sink,
            ProcessEventThrottleConfig {
                flush_interval: Duration::from_secs(60),
                max_batch: 100,
            },
        );
        let policy = ConvergencePolicy {
            max_internal_rounds: 10_000,
            allow_unbounded_internal_rounds: false,
            wall_clock_timeout: Some(Duration::ZERO),
        };
        let long = "z".repeat(crate::light_intent_rules::SHORT_QA_MAX_SCALARS + 1);
        let trace = run_minimal_turn_headless_with_full_swarm_events(
            &cort, &long, false, &pipe, &policy,
        );
        assert_eq!(trace.layer, CortexExecutionLayer::FullSwarmOrchestration);
        assert_eq!(trace.swarm_stop_reason, Some(SwarmRunStopReason::Timeout));
        assert!(rec.snapshot().iter().any(|e| {
            e.name == crate::ProcessEventNameV0::SwarmRunFinished
                && e.stop_reason == Some(SwarmRunStopReason::Timeout)
        }));
    }

    /// 模拟实现错误：无视皮层关状态，仍走全蜂群（仅测试模块内，用于 AC#4 可检索失败文案）。
    fn buggy_always_swarm(_rt: &CortexRuntime, _user_text: &str) -> MinimalTurnTrace {
        MinimalTurnTrace {
            layer: CortexExecutionLayer::FullSwarmOrchestration,
            process_events_emitted: 1,
            entered_multi_agent_handoff: true,
            swarm_stop_reason: Some(SwarmRunStopReason::Done),
            full_swarm_internal_rounds: 0,
            explicit_full_swarm_suppressed_by_cortex_off: false,
        }
    }

    fn assert_cortex_off_forbids_handoff_for_doc(trace: &MinimalTurnTrace) {
        assert!(
            !trace.entered_multi_agent_handoff,
            "【大脑皮层关路径错误】期望大脑皮层为「关」时不进入全蜂群；当前错误地走了「开」路径行为（entered_multi_agent_handoff=true）。请对照 CORTEX_OFF_SIMPLIFIED_MODE.md 修正大脑皮层开/关分支。"
        );
    }

    /// doc-ref: agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md §4 A4 — 模拟走错分支时 panic 文案须含「大脑皮层」「关/开」供 CI grep
    #[test]
    #[should_panic(expected = "大脑皮层")]
    fn cortex_off_wrong_branch_panics_with_cortex_keywords() {
        let rt = CortexRuntime::from_state(CortexState::with_enabled(false));
        let trace = buggy_always_swarm(&rt, "x");
        assert_cortex_off_forbids_handoff_for_doc(&trace);
    }
}
