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

## 简化模式（大脑皮层关，Story 1.4）

- 实现者语义与 headless 断言登记：[`docs/CORTEX_OFF_SIMPLIFIED_MODE.md`](docs/CORTEX_OFF_SIMPLIFIED_MODE.md)
- 最小 turn 桩（无 GUI）：[`src/minimal_turn.rs`](src/minimal_turn.rs) 中 `run_minimal_turn_headless(rt, user_text, explicit_full_swarm)`（FR19 轻量路由已接入）

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
