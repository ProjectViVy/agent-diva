//! FR20 / ADR-E：**全蜂群**路径上的 `ConvergencePolicy`、每步预算检查与 `swarm_run_*` 终局事件（Story 1.8）。
//!
//! # 语义
//!
//! - **`swarm_run_capped`：** 仅 **`SwarmRunStopReason::BudgetExceeded`**（内部轮次 / 等价预算触顶）。
//! - **`swarm_run_finished`：** `Done`、**`Timeout`**、**`Error`** 等其它终局原因（与架构 ADR-E 列举一致）。
//!
//! # NFR-P3
//!
//! 默认 **`allow_unbounded_internal_rounds == false`**；若开启「无上限」开关，编排仍须由 `is_done` 收敛，
//! **不得**将无上限内部对话作为线上默认唯一完成手段 — 见 crate `README.md` FR20 节。
//! 无上限模式下另有硬熔断 [`UNBOUNDED_CONVERGENCE_ITERATION_FUSE`]（release），防止 `is_done` 永不真时死循环。
//!
//! # 终局判定顺序（单点说明，Story 6.1）
//!
//! 每一轮迭代内 **依次**：
//! 1. 无上限模式迭代熔断（[`UNBOUNDED_CONVERGENCE_ITERATION_FUSE`] / 测试用缩短值）
//! 2. **`Done`** — `is_done(rounds_completed)` 为真（自然收敛优先于时间与轮次预算）
//! 3. **`Timeout`** — 配置了 [`ConvergencePolicy::wall_clock_timeout`] 且自循环起点起墙钟已耗尽
//! 4. **`BudgetExceeded`** — 有界模式下内部轮次已达 `max_internal_rounds` 且尚未 done
//! 5. 递增 `rounds_completed` 并进入下一轮
//!
//! 即：**`Done` 优先于 `Timeout`；`Timeout` 优先于 `BudgetExceeded`**。蜂群序曲（如 LLM handoff）的墙钟须由异步调用方另包
//! `timeout`；本函数只覆盖收敛循环本体。

use crate::process_events::{ProcessEventPipeline, ProcessEventV0, SwarmRunStopReason};
use std::time::{Duration, Instant};

/// 默认最大内部轮次（维护者调参改此常量并同步 `README` / `architecture.md` ADR-E）。
pub const DEFAULT_MAX_INTERNAL_ROUNDS: u32 = 256;

/// `allow_unbounded_internal_rounds == true` 且 `is_done` 永不真时的 **硬熔断** 迭代上限（防止本线程死循环 / 本地 DoS）。
pub const UNBOUNDED_CONVERGENCE_ITERATION_FUSE: u32 = 16_777_215;

#[cfg(test)]
const UNBOUNDED_FUSE_ACTIVE: u32 = 256;
#[cfg(not(test))]
const UNBOUNDED_FUSE_ACTIVE: u32 = UNBOUNDED_CONVERGENCE_ITERATION_FUSE;

/// 收敛策略：蜂群编排循环 **每步** 对照检查（FR20、NFR-P3）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvergencePolicy {
    /// 在 [`Self::allow_unbounded_internal_rounds`] 为 `false` 时，已完成的内部轮次 **达到或超过** 此值即触顶。
    pub max_internal_rounds: u32,
    /// **非生产默认**：为 `true` 时 **不**应用 `max_internal_rounds` 上限；须由 `is_done` 结束，否则可能死循环。
    pub allow_unbounded_internal_rounds: bool,
    /// **墙钟上限**（自进入 [`execute_full_swarm_convergence_loop`] 起算）。`None` 表示不启用（默认）。
    /// 触顶时终局为 [`SwarmRunStopReason::Timeout`] 并发射 `swarm_run_finished`（与 ADR-E / Story 6.1 一致）。
    pub wall_clock_timeout: Option<Duration>,
}

impl Default for ConvergencePolicy {
    fn default() -> Self {
        Self {
            max_internal_rounds: DEFAULT_MAX_INTERNAL_ROUNDS,
            allow_unbounded_internal_rounds: false,
            wall_clock_timeout: None,
        }
    }
}

/// Headless 桩用的默认 **完成谓词**：完成一轮内部推进后视为满足 done（非「仅靠无限对话」）。
#[must_use]
pub fn default_full_swarm_stub_is_done(rounds_completed: u32) -> bool {
    rounds_completed >= 1
}

fn emit_swarm_terminal(pipeline: &ProcessEventPipeline, reason: SwarmRunStopReason) {
    let msg: &'static str = match reason {
        SwarmRunStopReason::Done => "swarm run finished",
        SwarmRunStopReason::BudgetExceeded => "swarm run capped: budget exceeded",
        SwarmRunStopReason::Timeout => "swarm run finished: timeout",
        SwarmRunStopReason::Error => "swarm run finished: error",
    };
    let ev = match reason {
        SwarmRunStopReason::BudgetExceeded => ProcessEventV0::swarm_run_capped(msg),
        SwarmRunStopReason::Done | SwarmRunStopReason::Timeout | SwarmRunStopReason::Error => {
            ProcessEventV0::swarm_run_finished(reason, msg)
        }
    };
    pipeline.try_emit(ev);
}

/// FullSwarm 编排循环：**每步**先判定 `is_done`，再检查墙钟超时，再检查有界预算，最后递增已用轮次。
///
/// 顺序保证：当 `rounds_completed == max_internal_rounds` 且本步已满足完成谓词时，优先 **`Done`**，避免误发 **`BudgetExceeded`**（例如 `max_internal_rounds == 1` 且桩在一轮后 done）。
/// 当同时配置墙钟超时与轮次预算时，**`Timeout` 先于 `BudgetExceeded`**（见模块级「终局判定顺序」）。
///
/// - `rounds_completed`：已成功执行的内部轮次数（首次进入为 `0`）。
/// - 返回值：`(终局 StopReason, 终局时的 rounds_completed)`。
#[must_use]
pub fn execute_full_swarm_convergence_loop(
    policy: &ConvergencePolicy,
    pipeline: Option<&ProcessEventPipeline>,
    mut is_done: impl FnMut(u32) -> bool,
) -> (SwarmRunStopReason, u32) {
    let mut rounds_completed: u32 = 0;
    let wall_clock_start = policy
        .wall_clock_timeout
        .is_some()
        .then(Instant::now);
    loop {
        if policy.allow_unbounded_internal_rounds && rounds_completed >= UNBOUNDED_FUSE_ACTIVE {
            tracing::error!(
                target: "agent_diva_swarm::convergence",
                fuse = UNBOUNDED_FUSE_ACTIVE,
                "convergence iteration fuse: allow_unbounded_internal_rounds with is_done never true"
            );
            let r = SwarmRunStopReason::Error;
            if let Some(p) = pipeline {
                emit_swarm_terminal(p, r);
            }
            return (r, rounds_completed);
        }
        if is_done(rounds_completed) {
            let r = SwarmRunStopReason::Done;
            if let Some(p) = pipeline {
                emit_swarm_terminal(p, r);
            }
            return (r, rounds_completed);
        }
        if let (Some(limit), Some(started)) = (policy.wall_clock_timeout, wall_clock_start) {
            if started.elapsed() >= limit {
                let r = SwarmRunStopReason::Timeout;
                if let Some(p) = pipeline {
                    emit_swarm_terminal(p, r);
                }
                return (r, rounds_completed);
            }
        }
        if !policy.allow_unbounded_internal_rounds
            && rounds_completed >= policy.max_internal_rounds
        {
            let r = SwarmRunStopReason::BudgetExceeded;
            if let Some(p) = pipeline {
                emit_swarm_terminal(p, r);
            }
            return (r, rounds_completed);
        }
        rounds_completed = rounds_completed.saturating_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process_events::{
        ProcessEventNameV0, ProcessEventPipeline, ProcessEventThrottleConfig,
    };
    use crate::{recorder_sink, CortexRuntime};
    use std::sync::Arc;
    use std::time::Duration;

    #[test]
    fn default_policy_is_bounded() {
        let p = ConvergencePolicy::default();
        assert!(!p.allow_unbounded_internal_rounds);
        assert!(p.max_internal_rounds > 0);
        assert!(p.wall_clock_timeout.is_none());
    }

    #[test]
    fn budget_zero_never_done_emits_capped_with_budget_exceeded() {
        let cortex = Arc::new(CortexRuntime::new());
        let (rec, sink) = recorder_sink();
        let pipe = ProcessEventPipeline::new(
            cortex,
            sink,
            ProcessEventThrottleConfig {
                flush_interval: std::time::Duration::from_secs(60),
                max_batch: 100,
            },
        );
        let policy = ConvergencePolicy {
            max_internal_rounds: 0,
            allow_unbounded_internal_rounds: false,
            wall_clock_timeout: None,
        };
        let (reason, _) =
            execute_full_swarm_convergence_loop(&policy, Some(&pipe), |_| false);
        pipe.flush_pending();
        assert_eq!(reason, SwarmRunStopReason::BudgetExceeded);
        let names: Vec<_> = rec.snapshot().iter().map(|e| e.name).collect();
        assert!(
            names.contains(&ProcessEventNameV0::SwarmRunCapped),
            "expected swarm_run_capped, got {names:?}"
        );
        assert!(rec.snapshot().iter().any(|e| {
            e.name == ProcessEventNameV0::SwarmRunCapped
                && e.stop_reason == Some(SwarmRunStopReason::BudgetExceeded)
        }));
    }

    #[test]
    fn max_internal_rounds_one_allows_stub_done_before_cap() {
        let cortex = Arc::new(CortexRuntime::new());
        let (_rec, sink) = recorder_sink();
        let pipe = ProcessEventPipeline::new(
            cortex,
            sink,
            ProcessEventThrottleConfig {
                flush_interval: std::time::Duration::from_secs(60),
                max_batch: 100,
            },
        );
        let policy = ConvergencePolicy {
            max_internal_rounds: 1,
            allow_unbounded_internal_rounds: false,
            wall_clock_timeout: None,
        };
        let (reason, _) = execute_full_swarm_convergence_loop(
            &policy,
            Some(&pipe),
            default_full_swarm_stub_is_done,
        );
        pipe.flush_pending();
        assert_eq!(
            reason,
            SwarmRunStopReason::Done,
            "one allowed round must suffice for stub done at rounds_completed==1"
        );
    }

    #[test]
    fn stub_done_path_emits_finished_with_done() {
        let cortex = Arc::new(CortexRuntime::new());
        let (rec, sink) = recorder_sink();
        let pipe = ProcessEventPipeline::new(
            cortex,
            sink,
            ProcessEventThrottleConfig {
                flush_interval: std::time::Duration::from_secs(60),
                max_batch: 100,
            },
        );
        let policy = ConvergencePolicy::default();
        let (reason, _) = execute_full_swarm_convergence_loop(
            &policy,
            Some(&pipe),
            default_full_swarm_stub_is_done,
        );
        pipe.flush_pending();
        assert_eq!(reason, SwarmRunStopReason::Done);
        assert!(rec.snapshot().iter().any(|e| {
            e.name == ProcessEventNameV0::SwarmRunFinished
                && e.stop_reason == Some(SwarmRunStopReason::Done)
        }));
    }

    #[test]
    fn swarm_run_terminal_event_serde_round_trip() {
        let ev = ProcessEventV0::swarm_run_capped("capped");
        let j = serde_json::to_string(&ev).expect("ser");
        let back: ProcessEventV0 = serde_json::from_str(&j).expect("de");
        assert_eq!(ev, back);
        assert!(j.contains("stopReason"));
        assert!(j.contains("swarm_run_capped"));
    }

    #[test]
    fn unbounded_policy_hits_fuse_when_never_done() {
        let policy = ConvergencePolicy {
            max_internal_rounds: 0,
            allow_unbounded_internal_rounds: true,
            wall_clock_timeout: None,
        };
        let (reason, rounds) = execute_full_swarm_convergence_loop(&policy, None, |_| false);
        assert_eq!(reason, SwarmRunStopReason::Error);
        assert_eq!(rounds, super::UNBOUNDED_FUSE_ACTIVE);
    }

    #[test]
    fn wall_clock_timeout_zero_emits_finished_with_timeout() {
        let cortex = Arc::new(CortexRuntime::new());
        let (rec, sink) = recorder_sink();
        let pipe = ProcessEventPipeline::new(
            cortex,
            sink,
            ProcessEventThrottleConfig {
                flush_interval: std::time::Duration::from_secs(60),
                max_batch: 100,
            },
        );
        let policy = ConvergencePolicy {
            max_internal_rounds: 10_000,
            allow_unbounded_internal_rounds: false,
            wall_clock_timeout: Some(Duration::ZERO),
        };
        let (reason, rounds) =
            execute_full_swarm_convergence_loop(&policy, Some(&pipe), |_| false);
        pipe.flush_pending();
        assert_eq!(reason, SwarmRunStopReason::Timeout);
        assert_eq!(rounds, 0, "timeout before any internal round increment");
        assert!(rec.snapshot().iter().any(|e| {
            e.name == ProcessEventNameV0::SwarmRunFinished
                && e.stop_reason == Some(SwarmRunStopReason::Timeout)
        }));
    }

    #[test]
    fn done_wins_over_wall_clock_timeout_when_both_apply() {
        let policy = ConvergencePolicy {
            max_internal_rounds: 0,
            allow_unbounded_internal_rounds: false,
            wall_clock_timeout: Some(Duration::ZERO),
        };
        let (reason, _) =
            execute_full_swarm_convergence_loop(&policy, None, |_| true);
        assert_eq!(
            reason,
            SwarmRunStopReason::Done,
            "is_done must beat Timeout on same iteration"
        );
    }

    #[test]
    fn wall_clock_timeout_checked_before_budget_exceeded() {
        let policy_budget_only = ConvergencePolicy {
            max_internal_rounds: 0,
            allow_unbounded_internal_rounds: false,
            wall_clock_timeout: None,
        };
        let (r_cap, _) =
            execute_full_swarm_convergence_loop(&policy_budget_only, None, |_| false);
        assert_eq!(r_cap, SwarmRunStopReason::BudgetExceeded);

        let policy_with_clock = ConvergencePolicy {
            max_internal_rounds: 0,
            allow_unbounded_internal_rounds: false,
            wall_clock_timeout: Some(Duration::ZERO),
        };
        let (r_time, _) =
            execute_full_swarm_convergence_loop(&policy_with_clock, None, |_| false);
        assert_eq!(
            r_time,
            SwarmRunStopReason::Timeout,
            "same caps: wall clock evaluated before round budget"
        );
    }
}
