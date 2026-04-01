---
story_key: 2-4-run-telemetry-hint-fr22
story_id: "2.4"
epic: 2
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/planning-artifacts/prd.md
---

# Story 2.4：运行用量提示与开发者挂点（FR22、UX-DR5）

Status: done

## Story

As a **进阶用户 / 开发者**,  
I want **在可选位置看到本次运行的内部步数或预算提示**,  
So that **诊断「为何烧 token」不全黑盒（FR22）**。

## Acceptance Criteria

**Given** Story **1.x** 已暴露 **`RunTelemetrySnapshot`**（或等价 DTO），并经 Tauri **dev 命令、设置子区或折叠诊断抽屉** 之一对外（**本 story 内冻结一种**）  
**When** 用户打开该挂点（默认 **不** 打扰主聊天）  
**Then** 可见 **内部步数/阶段计数** 或 **超建议预算** 的 **琥珀非阻断** 提示  
**And** 实现可选用独立组件 **`RunTelemetryHint`**（见 UX 规格）；**feature-flag** 控制可接受  
**And** **不** 将内部 trace **默认写入** 用户 transcript（**NFR-R2**）

## Tasks / Subtasks

- [x] **依赖与契约**（AC：Given）  
  - [x] 确认 **Epic 1** 中 `RunTelemetrySnapshot`（或等价）的 **serde 类型、版本/白名单字段** 已与 `architecture.md` **ADR-E** 一致（`internal_step_count`、`phase_count`、可选 `over_suggested_budget`）  
  - [x] 选定 **唯一** 暴露面：**Tauri 开发者命令** / **设置子区** / **折叠诊断抽屉** 三选一，并在 README 或实现说明中 **写死**

- [x] **Rust / Tauri 侧**  
  - [x] 提供与选定暴露面匹配的 **invoke 或 poll**；**不** 把遥测摘要 **默认追加** 到用户可见 transcript 流（NFR-R2）

- [x] **GUI（`agent-diva-gui`）**  
  - [x] 实现或接入 **`RunTelemetryHint`**（或等价）：一行摘要 + 可选展开；**琥珀** 警告，与 **错误红** 区分（UX 规格）  
  - [x] 默认 **关闭** 或仅在 **开发者/高级** 路径展示（**UX-DR5**）  
  - [x] （可选）**feature-flag** 控制开发者可见性

- [x] **验证**  
  - [x] 走查或自动化：**打开挂点后** 能读到 **步数/阶段或超预算** 之一；主聊天区 **无** 默认遥测污染 transcript  
  - [x] 与 PRD **旅程六**、反模式「token 黑盒」相关的 **至少一条** 验收路径（可与 `ux-design-specification.md` 反旅程验收对齐）

## Dev Agent Record

### Implementation Plan

- 在 `agent_diva_core` 定义 `RunTelemetrySnapshotV0`（serde camelCase、schema_version v0、白名单字段），`AgentEvent::RunTelemetry` 承载；代理 turn 结束在 `loop_turn` 中根据迭代次数与是否触顶 `max_iterations` 发射，经 Manager SSE `run_telemetry` 与 `final` 分轨。
- Tauri：`send_message` 解析 `run_telemetry` 写入内存缓存；`get_run_telemetry_snapshot` invoke 供设置页读取。
- GUI：**设置 → 高级** + `RunTelemetryHint.vue`；`localStorage` feature-flag；README 冻结暴露面。

### Debug Log

- 编译 `agent-diva-agent` 时 `BusSwarmProcessSink` 的 `#[derive(Debug)]` 与 `MessageBus` 冲突，已移除该 derive（与 Story 2.3 结构体一致、非本 story 范围外的最小修复）。

### Completion Notes

- ✅ `cargo test -p agent-diva-core -p agent-diva-swarm` 通过；`cargo check -p agent-diva-agent -p agent-diva-manager -p agent-diva-gui` 通过；`vue-tsc --noEmit` 通过。
- ✅ 验收路径：用户在桌面端完成一次聊天后打开 **设置 → 高级**，勾选开发者遥测并刷新，可见步数/阶段；超预算时 `RunTelemetryHint` 琥珀样式；主聊天 SSE 仍仅为 `delta`/`final` 等，会话持久化未写入遥测。

## File List

- `agent-diva/agent-diva-core/src/bus/run_telemetry.rs`（新建）
- `agent-diva/agent-diva-core/src/bus/mod.rs`
- `agent-diva/agent-diva-core/src/bus/events.rs`
- `agent-diva/agent-diva-agent/src/agent_loop/loop_turn.rs`
- `agent-diva/agent-diva-agent/src/swarm_process_bus.rs`
- `agent-diva/agent-diva-manager/src/handlers.rs`
- `agent-diva/agent-diva-swarm/src/run_telemetry.rs`（新建）
- `agent-diva/agent-diva-swarm/src/lib.rs`
- `agent-diva/agent-diva-cli/src/client.rs`
- `agent-diva/agent-diva-cli/src/main.rs`
- `agent-diva/agent-diva-gui/src-tauri/src/lib.rs`
- `agent-diva/agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva/agent-diva-gui/src/api/runTelemetry.ts`（新建）
- `agent-diva/agent-diva-gui/src/components/RunTelemetryHint.vue`（新建）
- `agent-diva/agent-diva-gui/src/components/settings/AdvancedSettings.vue`（新建）
- `agent-diva/agent-diva-gui/src/components/SettingsView.vue`
- `agent-diva/agent-diva-gui/src/components/settings/SettingsDashboard.vue`
- `agent-diva/agent-diva-gui/src/components/NormalMode.vue`
- `agent-diva/agent-diva-gui/src/locales/en.ts`
- `agent-diva/agent-diva-gui/src/locales/zh.ts`
- `agent-diva/agent-diva-gui/README.md`

## Change Log

- **2026-03-31：** Story 2.4 — FR22 运行遥测 DTO、SSE `run_telemetry`、Tauri 缓存与 invoke、设置「高级」+ `RunTelemetryHint` + feature-flag；蜂群 `run_telemetry_from_minimal_turn_trace` 契约测试。

## Dev Notes

### 前置依赖

| 依赖 | 说明 |
|------|------|
| **Story 1.x 遥测 DTO** | `RunTelemetrySnapshot`（或等价）须已由后端/Tauri 契约暴露；本 story **消费** 该 DTO，**不** 重新定义业务真相源 |
| **Epic 2 内序** | 建议与 **2.3**（过程反馈 UI）及 **1.5、1.8** 事件衔接兼容；同 Epic 内仅依赖 **更小序号** 故事（见 `epics.md` 故事依赖说明） |

### ADR-E — `RunTelemetrySnapshot`（FR22）

与 `architecture.md` **ADR-E** 对齐：

- **字段（示意）：** `internal_step_count`、`phase_count`、可选 `over_suggested_budget`  
- **暴露：** **Tauri 开发者命令**、**设置子区** 或 **poll**（与 Story AC 三选一冻结一致）  
- **隐私：** **不** 默认写入用户 transcript（**NFR-R2**）  
- **契约：** 字段 **版本化 + 白名单**（**NFR-I2**）

### UX-DR5 与 `RunTelemetryHint`

| 来源 | 要点 |
|------|------|
| **UX-DR5** | MVP 成本/用量以 **开发者向挂点** 为主（设置、折叠调试区、次要文案）；**非阻断琥珀** 可表示超建议预算；完整计费 UI **Post-MVP** |
| **`RunTelemetryHint`** | **不阻断聊天**；内容：内部步数、阶段计数或「超建议预算」；位置候选：设置 → 高级、聊天页折叠「诊断」、神经系统详情脚标 — **与「暴露面三选一」一并冻结** |

### NFR-R2（禁止 transcript 泄漏）

**内部 trace** 与用户 transcript **默认分轨**；调试/遥测内容 **不得** 默认混入用户可见对话记录。实现时须审计：流式通道、持久化会话、任何「复制到剪贴板/导出」路径。

### 追溯

| 项 | 文档 |
|----|------|
| Story 原文 | `_bmad-output/planning-artifacts/epics.md` — Story 2.4 |
| ADR-E / 遥测 | `_bmad-output/planning-artifacts/architecture.md` — ADR-E，`RunTelemetrySnapshot` |
| UX | `_bmad-output/planning-artifacts/ux-design-specification.md` — UX-DR5，`RunTelemetryHint`，旅程六 |
| FR22 / NFR-R2 | `_bmad-output/planning-artifacts/prd.md` |

### Review Findings

<!-- code review 2026-03-31：无 git diff，按 File List 源码三联审查（Blind / Edge / Auditor） -->

- [x] [Review][Patch] `run_telemetry` SSE 在序列化失败时不应下发 `"{}"`（破坏 ADR-E schema 契约，客户端静默无法更新缓存） [`agent-diva/agent-diva-manager/src/handlers.rs`] — 已修复：失败时改为 SSE `comment("run_telemetry_omitted")`，不再发送非法 JSON
