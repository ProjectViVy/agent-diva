//! 最小用户 turn 路径（headless，Story 1.4 + FR19 Story 1.7）：与 **大脑皮层开/关** 及 **执行分层** 对齐的可测桩。
//!
//! 不发起网络请求、不依赖 GUI。语义登记见 `docs/CORTEX_OFF_SIMPLIFIED_MODE.md`。轻量路由规则见 [`crate::light_intent_rules`]。

use crate::{resolve_execution_tier, CortexRuntime, ExecutionTier};

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
    /// 简化模式下中间「过程类」事件计数（当前 OFF 路径为 0）。
    pub process_events_emitted: u32,
    /// 是否进入多代理 handoff 链（OFF 必须为 false）。
    pub entered_multi_agent_handoff: bool,
}

/// 执行与 GUI/网络无关的最小 turn，读取 [`CortexRuntime`] 真相源。
///
/// `explicit_full_swarm` 表示用户/设置 **显式** 选择深度编排；`false` 时按 FR19 与 [`resolve_execution_tier`] 路由。
#[must_use]
pub fn run_minimal_turn_headless(
    rt: &CortexRuntime,
    user_text: &str,
    explicit_full_swarm: bool,
) -> MinimalTurnTrace {
    let enabled = rt.snapshot().enabled;
    if !enabled {
        return MinimalTurnTrace {
            layer: CortexExecutionLayer::Simplified,
            process_events_emitted: 0,
            entered_multi_agent_handoff: false,
        };
    }
    match resolve_execution_tier(user_text, true, explicit_full_swarm) {
        ExecutionTier::Light => MinimalTurnTrace {
            layer: CortexExecutionLayer::LightPath,
            process_events_emitted: 0,
            entered_multi_agent_handoff: false,
        },
        ExecutionTier::FullSwarm => MinimalTurnTrace {
            layer: CortexExecutionLayer::FullSwarmOrchestration,
            process_events_emitted: 1,
            entered_multi_agent_handoff: true,
        },
    }
}

#[cfg(test)]
mod cortex_off_tests {
    use super::*;
    use crate::{CortexRuntime, CortexState};

    /// doc-ref: agent-diva-swarm/docs/CORTEX_OFF_SIMPLIFIED_MODE.md §2 多参与者蜂群
    #[test]
    fn cortex_off_minimal_turn_skips_full_swarm_handoff() {
        let rt = CortexRuntime::from_state(CortexState::with_enabled(false));
        let trace = run_minimal_turn_headless(&rt, "hello", false);
        assert_eq!(trace.layer, CortexExecutionLayer::Simplified);
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

    /// 模拟实现错误：无视皮层关状态，仍走全蜂群（仅测试模块内，用于 AC#4 可检索失败文案）。
    fn buggy_always_swarm(_rt: &CortexRuntime, _user_text: &str) -> MinimalTurnTrace {
        MinimalTurnTrace {
            layer: CortexExecutionLayer::FullSwarmOrchestration,
            process_events_emitted: 1,
            entered_multi_agent_handoff: true,
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
