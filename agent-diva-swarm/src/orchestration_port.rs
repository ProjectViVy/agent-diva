//! Story 5.4：外部编排适配端口（SPI stub）。默认实现与当前 headless FullSwarm **收敛循环**一致。
//!
//! **序曲（多角色 LLM）** 仍在 `agent-diva-agent` 的 `run_swarm_deliberation_prelude`；本端口描述 **swarm crate 内**
//! 在 [`crate::ExecutionTier::FullSwarm`] 之后可替换的 **收敛 / 终局** 边界，供 Shannon、Python 宿主等评估接入。
//!
//! **ADR-A：** 本 crate **不得**依赖 `agent-diva-meta`；外部宿主由 runtime / gateway **组合层**注入实现。

use crate::convergence::{
    default_full_swarm_stub_is_done, execute_full_swarm_convergence_loop, ConvergencePolicy,
};
use crate::process_events::ProcessEventPipeline;
use crate::SwarmRunStopReason;

/// FullSwarm 编排输入快照（v0 DTO stub，供 ADR 与宿主对齐；不持有全文以降低重复分配）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwarmOrchestrationInputV0 {
    /// 用户可见文本字节长度（UTF-8）。
    pub user_text_len: usize,
    /// 是否显式请求全蜂群深度。
    pub explicit_full_swarm: bool,
}

impl SwarmOrchestrationInputV0 {
    /// 从当前 turn 上下文构造输入快照。
    #[must_use]
    pub fn from_turn(user_text: &str, explicit_full_swarm: bool) -> Self {
        Self {
            user_text_len: user_text.len(),
            explicit_full_swarm,
        }
    }
}

/// FullSwarm 编排输出（与 [`execute_full_swarm_convergence_loop`] 返回值对齐）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwarmOrchestrationOutcome {
    pub stop_reason: SwarmRunStopReason,
    pub rounds_completed: u32,
}

/// 端口：在 **FullSwarm** 路径上抽象「有界收敛 + 可选 `swarm_run_*` 终局事件」。
///
/// 默认实现 [`BuiltinSwarmOrchestrationPort`] = 当前 Rust 内置桩（`default_full_swarm_stub_is_done`）。
/// 失败降级策略见 `docs/ADR_ORCHESTRATION_SPI_SHANNON_BRIDGE.md`（组合层负责回退内置序曲 / Light，不在此 trait 内耦合 meta）。
pub trait SwarmOrchestrationPort {
    /// 执行收敛阶段：`pipeline == None` 时不发射过程事件（FR20）。
    fn run_convergence(
        &self,
        input: &SwarmOrchestrationInputV0,
        policy: &ConvergencePolicy,
        pipeline: Option<&ProcessEventPipeline>,
    ) -> SwarmOrchestrationOutcome;
}

/// 内置端口：Rust 收敛循环 + 默认 `is_done` 桩。
#[derive(Debug, Clone, Copy, Default)]
pub struct BuiltinSwarmOrchestrationPort;

impl SwarmOrchestrationPort for BuiltinSwarmOrchestrationPort {
    fn run_convergence(
        &self,
        _input: &SwarmOrchestrationInputV0,
        policy: &ConvergencePolicy,
        pipeline: Option<&ProcessEventPipeline>,
    ) -> SwarmOrchestrationOutcome {
        let (stop_reason, rounds_completed) = execute_full_swarm_convergence_loop(
            policy,
            pipeline,
            default_full_swarm_stub_is_done,
        );
        SwarmOrchestrationOutcome {
            stop_reason,
            rounds_completed,
        }
    }
}

/// 全工作区默认使用的内置编排端口（单例零大小类型）。
pub const DEFAULT_SWARM_ORCHESTRATION_PORT: BuiltinSwarmOrchestrationPort =
    BuiltinSwarmOrchestrationPort;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convergence::{
        default_full_swarm_stub_is_done, execute_full_swarm_convergence_loop,
    };
    use crate::process_events::{ProcessEventPipeline, ProcessEventThrottleConfig};
    use crate::{recorder_sink, CortexRuntime, CortexState};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn builtin_port_matches_direct_convergence_without_pipeline() {
        let policy = ConvergencePolicy::default();
        let direct = execute_full_swarm_convergence_loop(
            &policy,
            None,
            default_full_swarm_stub_is_done,
        );
        let input = SwarmOrchestrationInputV0::from_turn("hello", false);
        let out = DEFAULT_SWARM_ORCHESTRATION_PORT.run_convergence(&input, &policy, None);
        assert_eq!(out.stop_reason, direct.0);
        assert_eq!(out.rounds_completed, direct.1);
    }

    #[test]
    fn builtin_port_matches_direct_convergence_with_pipeline() {
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
        let direct = execute_full_swarm_convergence_loop(
            &policy,
            Some(&pipe),
            default_full_swarm_stub_is_done,
        );
        let input = SwarmOrchestrationInputV0::from_turn("x", true);
        let out = DEFAULT_SWARM_ORCHESTRATION_PORT.run_convergence(&input, &policy, Some(&pipe));
        assert_eq!(out.stop_reason, direct.0);
        assert_eq!(out.rounds_completed, direct.1);
        pipe.flush_pending();
        assert!(rec.snapshot().iter().any(|e| {
            e.name == crate::ProcessEventNameV0::SwarmRunCapped
        }));
    }
}
