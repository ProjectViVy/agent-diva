# agent-diva-swarm

蜂群编排相关 Rust crate，位于 agent-diva workspace 内：`agent-diva/agent-diva-swarm/`（与 `_bmad-output/planning-artifacts/architecture.md` 中 Project Structure 示例一致）。

## 规划与架构（仓库根相对路径）

- 产品需求：[`_bmad-output/planning-artifacts/prd.md`](../../_bmad-output/planning-artifacts/prd.md)
- 架构说明：[`_bmad-output/planning-artifacts/architecture.md`](../../_bmad-output/planning-artifacts/architecture.md)

## ADR-A（Swarm 与 Meta 边界）

编排 swarm 实现 **不得** 依赖 `agent-diva-meta`；Meta 仅在 runtime / gateway 组合层边界触发。本 crate 的 `Cargo.toml` 中 **不得** 出现 `agent-diva-meta`。

后续若在 CI 中加强门禁，可对 `agent-diva-swarm` 运行 `cargo tree -p agent-diva-swarm` 并断言输出中不包含 `agent-diva-meta`（脚本可放在 `agent-diva/scripts/` 或工作区 `justfile` 中，本 story 不强制启用）。

## 大脑皮层（Cortex）状态、持久化与 FR14

- **权威位置：** `CortexState` / `CortexRuntime`（`src/cortex.rs`）为 **Rust 侧单一真相源**；与 **FR14** 一致 — GUI 不长期自持另一份持久权威状态，仅通过后续 gateway / Tauri 契约消费（见 `architecture.md` 运行时真相源）。
- **持久化边界（当前）：** **仅进程内内存**。进程退出即丢失；**未**引入本特性专用 DB。若将来需要跨重启恢复，须在 ADR/实现说明中写明并沿用既有配置或存储实践（对齐架构「不新增专用 DB」）。
- **默认值：** `enabled` 默认 **`true`**（常量 `CORTEX_DEFAULT_ENABLED`），与 PRD 主路径「蜂群层默认可用、可显式关闭」一致；变更须同步代码注释与测试。

**GUI / Gateway 契约 v0：** Tauri command 与事件清单、JSON 形状、白名单与变更流程见仓库根 [`docs/swarm-cortex-contract-v0.md`](../../docs/swarm-cortex-contract-v0.md)（Story 1.3）。

**Story 1.2 / 1.3 契约片段路径（本 story 基线）：**

| 故事 | 路径 |
|------|------|
| 1.2 皮层状态与持久化 | 本文档上一节 + [`src/cortex.rs`](src/cortex.rs) |
| 1.3 Gateway 同步 DTO | [`docs/swarm-cortex-contract-v0.md`](../../docs/swarm-cortex-contract-v0.md) |

## FR19（执行分层 / 轻量路径，Story 1.7）

- **轻量意图唯一真相源：** [`src/light_intent_rules.rs`](src/light_intent_rules.rs) — 显式 skill 风格与短问答阈值；实现与测试 **仅** 通过 `is_light_intent` / `SHORT_QA_MAX_SCALARS` 等本模块 API 引用。
- **Light vs FullSwarm 路由：** [`src/execution_tier.rs`](src/execution_tier.rs) — `resolve_execution_tier`；皮层 ON **不得单独** 将轻量类升为 FullSwarm（须显式深度选择）。
- **轻量路径上限与触顶原因：** [`src/light_path_limits.rs`](src/light_path_limits.rs) — `LIGHT_PATH_MAX_WALL_MS`、`LIGHT_PATH_MAX_INTERNAL_STEPS`、`LightPathStopReason`。
- **ADR 冻结入口：** [`docs/adr-e-fr19-execution-tier.md`](../../docs/adr-e-fr19-execution-tier.md)（工作区根 `docs/`）。

## FR20 / NFR-P3（收敛策略与终局语义，Story 1.8 + Story 6.1 墙钟超时）

- **策略类型：** [`src/convergence.rs`](src/convergence.rs) — `ConvergencePolicy`（`max_internal_rounds` 默认见 `DEFAULT_MAX_INTERNAL_ROUNDS` = **256**；`allow_unbounded_internal_rounds` 默认 **`false`**；`wall_clock_timeout` 默认 **`None`**，为 `Some(duration)` 时在收敛循环内产生 `SwarmRunStopReason::Timeout`）。编排循环 **每步** 对照策略检查；**禁止**将无上限内部多轮对话作为 **默认唯一** 完成手段（与 `_bmad-output/planning-artifacts/architecture.md` **ADR-E**、**NFR-P3** 一致）。
- **终局判定顺序（单点）：** 见 `convergence.rs` 模块文档 — 摘要：**`Done` > `Timeout`（墙钟）> `BudgetExceeded`（轮次）**；无上限熔断与 `Error` 见源码。异步蜂群 **序曲**（LLM 调用）的墙钟由调用方另包超时，本策略字段仅覆盖 `execute_full_swarm_convergence_loop`。
- **终局原因（ADR-E `StopReason` 在过程事件中的名称）：** `SwarmRunStopReason` — `Done` | `BudgetExceeded` | `Timeout` | `Error`（`src/process_events.rs`）。
- **白名单事件：** `swarm_run_finished`（正常结束或 `Timeout` / `Error` 终局）与 `swarm_run_capped`（**仅** `BudgetExceeded` 触顶）；载荷字段见 [`docs/process-events-v0.md`](docs/process-events-v0.md)。
- **API：** `execute_full_swarm_convergence_loop`；headless 集成：`run_minimal_turn_headless`（无管道时不发射 `swarm_run_*`，仍执行有界循环）与 `run_minimal_turn_headless_with_full_swarm_events`（经 `ProcessEventPipeline` 发射终局事件）。

## FullSwarm 序曲配置（Story 5.1，NFR-I2）

**维护者单点路径（冻结）：** 在 **工作区根目录**（与 `AgentLoop` 的 `workspace` 一致）放置下列文件之一，按顺序尝试加载：

1. `swarm-prelude.toml`（优先）
2. `swarm-prelude.yaml`
3. `swarm-prelude.yml`

**未提供文件**时，行为与阶段 A 硬编码 **逐字等价**（两角色、768 token、0.4/0.5 温度、相同过程事件 phase id/文案）。

**解析失败**时：`tracing::warn!` 并 **安全回退** 至上述默认（不中断主循环）。

**Rust API：** [`prelude_config.rs`](src/prelude_config.rs) — `load_swarm_prelude_config_from_workspace`、`SwarmPreludeConfig`；由 `agent-diva-agent` 在 FullSwarm turn 内调用。

**Handoff 检查点（Story 5.3）：** [`docs/handoff-checkpoint-v0.md`](docs/handoff-checkpoint-v0.md) — `SwarmHandoffCheckpointV0`（`src/handoff_checkpoint.rs`）；序曲中途失败时记录最后成功步，经 `tracing` 目标 `agent_diva_agent::prelude` 输出，**不** 进入 `ProcessEventV0` 白名单 v0。

### 示例：`swarm-prelude.toml`

```toml
schema_version = 1
enabled = true
# 序曲内最多几次 LLM 调用（角色步数）；超过则发射 swarm_phase_changed：swarm_prelude_round_cap
max_prelude_rounds = 2

[[roles]]
phase_id = "swarm_peer_planner"
phase_label = "蜂群 · 规划代理正在整理思路"
system_prompt = "你是多智能体蜂群中的「规划代理」。只输出条理清晰的要点与建议步骤，不要寒暄，不要自称 AI。"
input = "original_user"        # 或 previous_output
max_tokens = 768
temperature = 0.4
summary_section_title = "【规划摘要】"

[[roles]]
phase_id = "swarm_peer_critic"
phase_label = "蜂群 · 批评代理正在回应规划"
system_prompt = "你是蜂群中的「批评/风险代理」。针对上一条「规划代理」的输出，指出盲区、风险与需补充的验证点。用简洁列表。"
input = "previous_output"
max_tokens = 768
temperature = 0.5
summary_section_title = "【批评与补充】"

[merge_phase]
enabled = true
phase_id = "swarm_peer_merge"
phase_label = "蜂群 · 内部交流已收敛，主代理将综合答复"
```

关闭序曲（直接进入主 ReAct，仍遵守 FR19/FR20 路由）：

```toml
schema_version = 1
enabled = false
```

**`max_prelude_rounds` 触顶与 `merge_phase`（产品语义，2026-03-31 评审裁定）：** 若在角色链中途达到 `max_prelude_rounds`，会先发射 `swarm_phase_changed`，其 `phase_id` 为 **`swarm_prelude_round_cap`**（见 `run_swarm_deliberation_prelude`）。**随后若 `[merge_phase].enabled = true`，仍会发射配置的 merge 阶段事件**（默认 `phase_id = swarm_peer_merge`）。含义是：序曲阶段在预算内结束或触顶后，**一律**进入「主代理综合答复」前的同一道过程门闸；触顶 **不** 表示内部角色已全部跑完，观测方应结合 `swarm_prelude_round_cap` 与摘要内容区分「完整序曲」与「截断序曲」。

### 与 `architecture.md` ADR-E 名称对照

| ADR-E（摘录） | 本 crate 首版实现 |
|---------------|-------------------|
| `ConvergencePolicy` | `ConvergencePolicy`（`convergence.rs`） |
| `StopReason` | `SwarmRunStopReason`（过程事件 DTO 侧，语义一致） |
| `swarm_run_finished` / `swarm_run_capped` | `ProcessEventNameV0::SwarmRunFinished` / `SwarmRunCapped`（wire：`snake_case`） |

## 简化模式（大脑皮层关，Story 1.4）

- 实现者语义与 headless 断言登记：[`docs/CORTEX_OFF_SIMPLIFIED_MODE.md`](docs/CORTEX_OFF_SIMPLIFIED_MODE.md)
- 最小 turn 桩（无 GUI）：[`src/minimal_turn.rs`](src/minimal_turn.rs) 中 `run_minimal_turn_headless` / `run_minimal_turn_headless_with_full_swarm_events`（FR19 轻量路由 + FR20 全蜂群收敛已接入）

## 过程事件（FR2 发射侧，Story 1.5）

- **类型与节流：** [`src/process_events.rs`](src/process_events.rs) — `ProcessEventV0` / `ProcessEventPipeline`（皮层门控 + 批处理默认 100ms / 32 条；工具起止立即 flush）。
- **白名单与字段：** [`docs/process-events-v0.md`](docs/process-events-v0.md)（NFR-I2）。
- **皮层关不发射：** [`docs/PROCESS_EVENTS_CORTEX_OFF.md`](docs/PROCESS_EVENTS_CORTEX_OFF.md)。
- **Agent 接线：** `agent-diva-agent` 的 `AgentLoop::with_process_event_pipeline` 在迭代与工具路径上调用 `try_emit`；Tauri `emit` 适配留待网关/GUI 组合层。

## 设计文档（同仓库另一目录）

详细设计稿仍位于 **`agent-diva-swarm/docs/`**（与 `agent-diva/agent-diva-swarm`  crate 名称同前缀，物理路径不同，避免混淆）：

- [`agent-diva-swarm/docs/ARCHITECTURE_DESIGN.md`](../../agent-diva-swarm/docs/ARCHITECTURE_DESIGN.md)

## 构建

在 `agent-diva` 目录下：

```bash
cargo clippy -p agent-diva-swarm -- -D warnings
cargo test -p agent-diva-swarm
```
