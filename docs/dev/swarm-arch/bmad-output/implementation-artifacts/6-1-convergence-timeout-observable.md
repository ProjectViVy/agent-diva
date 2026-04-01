---
story_key: 6-1-convergence-timeout-observable
story_id: "6.1"
epic: 6
status: done
generated: "2026-03-31T20:30:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/implementation-artifacts/deferred-work.md
---

# Story 6.1：收敛循环墙钟/外部超时与 Timeout 终局可观测

详见 `epics.md` **Epic 6 / Story 6.1** 全文 AC。  
**关闭：** `deferred-work.md` 中 Story **1-8** 关于 `SwarmRunStopReason::Timeout` 的评审挂起项。

## Story

As a **开发者**,  
I want **在真实编排路径上能为 `execute_full_swarm_convergence_loop`（或等价）注入墙钟/外部超时并产生 `SwarmRunStopReason::Timeout` + `swarm_run_finished`**,  
So that **FR20 / ADR-E 的 Timeout 语义不是「仅有枚举无路径」（关闭 Story 1.8 评审 deferred）**。

## Acceptance Criteria

- [x] **Given** 可配置或测试注入的超时（`ConvergencePolicy::wall_clock_timeout`）  
- [x] **When** 超时在收敛循环执行中触发（`Duration::ZERO` 单测 / 集成测）  
- [x] **Then** 可观测 **Timeout** 终局（`swarm_run_finished` + `stop_reason == Timeout`）且 **有** 自动化测试覆盖  
- [x] **And** 与 `BudgetExceeded` / `Done` 的优先级在 `convergence.rs` 模块文档与 `agent-diva-swarm/README.md` FR20 节单点说明  

## Tasks / Subtasks

- [x] 扩展 `ConvergencePolicy`：`wall_clock_timeout: Option<Duration>`（默认 `None`，行为与旧版一致）
- [x] `execute_full_swarm_convergence_loop`：每轮在 `is_done` 之后、`BudgetExceeded` 之前检查墙钟；发射 `swarm_run_finished` / `Timeout`
- [x] 单元测试：`Timeout` 事件、`Done` 优先于 `Timeout`、`Timeout` 优先于 `BudgetExceeded`
- [x] 集成测试：`run_minimal_turn_headless_with_full_swarm_events` + 管道 recorder 断言
- [x] 文档：`convergence.rs` 模块级顺序说明 + README FR20；`deferred-work.md` 挂起项结案

## Dev Notes

- 序曲（LLM handoff）墙钟不在本故事内实现；由异步调用方包 `timeout`（已在模块文档说明）。
- 默认策略未设置墙钟，Agent 路径 `ConvergencePolicy::default()` 无回归。

## Dev Agent Record

### Implementation Plan

1. 在收敛循环入口记录 `Instant`（仅当配置了 `wall_clock_timeout`）。
2. 判定顺序：熔断 → `Done` → `Timeout` → `BudgetExceeded` → 递增轮次。

### Debug Log

- （无）

### Completion Notes

- ✅ 实现 `wall_clock_timeout` 与 `SwarmRunStopReason::Timeout` 终局发射路径；`cargo test -p agent-diva-swarm` 全绿；`cargo clippy -p agent-diva-swarm -D warnings` 通过；`agent-diva-agent` 编译通过。

## File List

- `agent-diva/agent-diva-swarm/src/convergence.rs`
- `agent-diva/agent-diva-swarm/src/minimal_turn.rs`
- `agent-diva/agent-diva-swarm/README.md`
- `_bmad-output/implementation-artifacts/deferred-work.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/6-1-convergence-timeout-observable.md`

## Change Log

- 2026-03-31：Story 6.1 — 收敛墙钟超时、`swarm_run_finished`/`Timeout` 可观测与测试；文档与 deferred 结案。

### Review Findings

- [x] [Review][Patch] `default_policy_is_bounded` 应断言 `wall_clock_timeout == None`，防止默认策略回归时新字段被误改 [`convergence.rs`](../../agent-diva/agent-diva-swarm/src/convergence.rs) — fixed 2026-03-31
- [x] [Review][Defer] 墙钟仅在每轮迭代边界采样；`is_done` 闭包执行期间无法触发 `Timeout` — 与同步收敛循环模型一致，序曲墙钟已由模块/README 划出 [`convergence.rs`](../../agent-diva/agent-diva-swarm/src/convergence.rs):102-141 — deferred, pre-existing

## Status

done
