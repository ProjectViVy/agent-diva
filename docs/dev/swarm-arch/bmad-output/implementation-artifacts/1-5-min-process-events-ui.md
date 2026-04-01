---
story_key: 1-5-min-process-events-ui
story_id: "1.5"
epic: 1
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
---

# Story 1.5：最小过程事件发射（供 UI 订阅）

Status: done

## Story

作为 **后续 Epic 中的 GUI 消费方**，  
我想要 **在后端发出可订阅的最小过程事件**（阶段变更或工具起止等），  
以便 **FR2 的数据源在 Rust 侧真实存在**，且与 gateway/皮层状态契约一致。

**前置依赖：** 须先完成 **Story 1.2**（皮层状态）、**1.3**（Gateway 与蜂群状态同步契约）、**1.4**（关皮层简化模式与无头测试），再接入本故事；事件不得与 **FR14 单一真相源** 或 **1.3 DTO** 矛盾。

## Acceptance Criteria

1. **Given** 大脑皮层为 **开（ON）**，且当前执行路径包含 **工具调用或可分阶段推进** 的请求  
   **When** 运行时向前推进（阶段变化、工具开始/结束等）  
   **Then** 在约定的事件总线、channel 或流式通道上可出现 **至少一种** 非「仅最终用户文本」的条目（满足 FR2 发射侧）

2. **And** 对外暴露的事件类型与 payload 字段落在 **文档化白名单** 内，schema **serde 可版本化**、稳定可列清单（**NFR-I2**）；禁止「general」式无限域膨胀

3. **And** 当事件产生速率可能过高时，实现须提供 **节流或批处理钩子**（例如合并窗口、丢弃/采样策略之一），并在代码或维护者文档中写明默认行为，**不得**单独拖垮主聊天流式路径（**NFR-P2**；与架构「过程更新批处理/节流」一致）

4. **And** 当大脑皮层为 **关** 时，本故事不要求完整 UI，但须与 **1.4** 登记的简化语义 **一致**：过程事件 **不发射** 或 **显式降级** 行为须在实现说明中写清，避免与 FR3 冲突

## Tasks / Subtasks

- [x] **冻结 v0 事件清单与载荷形状**（AC: #2）  
  - [x] 在 ADR 或 `docs`/crate 片段中列出 **白名单事件名**（建议遵循架构：`snake_case`、如 `swarm_phase_changed`、工具起止类命名与现有 channel 流式约定对齐）  
  - [x] 载荷 **小而稳定**：阶段 id、简短 message、可选结构化 id；大块内容用引用而非塞满事件体（见 `architecture.md` 过程可视事件约定）  
  - [x] DTO 含 **版本字段**，与 **1.3** 契约风格一致

- [x] **在皮层 ON + 工具/阶段路径上接线发射**（AC: #1）  
  - [x] 在 gateway / 蜂群适配层或既有总线延伸点 **单一出口** 发出事件，避免双栈并行（架构：新增过程事件须 ADR 指定 channel 或回调一种）  
  - [x] 至少覆盖 **一种**：**阶段类** 或 **工具起止类**（epics 原文：阶段或工具起止等）

- [x] **实现 NFR-P2 节流钩子**（AC: #3）  
  - [x] 提供可测试的 **批处理/节流层**（例如每 100–250ms 合并一次为架构示例，具体数值由实现选定并文档化）  
  - [x] Tauri 侧若涉及 emit，约定 **不长时间阻塞 UI 线程**（与 NFR-P1/P2 及 UX-DR3 对齐）

- [x] **皮层 OFF 行为**（AC: #4）  
  - [x] 与 Story **1.4** 文档交叉引用：OFF 时无事件或降级策略

- [x] **验证**（AC: #1–#4）  
  - [x] **无 GUI 测试**：皮层 **开**、桩工具/阶段推进 → 断言 **至少一条** 白名单事件到达订阅端或测试探针  
  - [x] 可选：节流单元测试（高频率源 → 下游接收不超过某上限或合并批次符合预期）

## Dev Notes

### Epic 1 上下文

本 Epic 在后端确立皮层真相源、简化模式、**最小过程事件**、执行分层与遥测等。**本故事只交付 FR2 的数据源（发射侧）**，完整聊天主页过程条 UI 属 **Epic 2 / Story 2.3**；神经系统视图与过程数据同源要求见 **Story 3.2**。

### 架构合规（必须遵守）

| 主题 | 要求 | 来源 |
|------|------|------|
| **NFR-I2** | 集成面 **可列清单**：白名单事件与字段；DTO serde 可版本化 | `architecture.md` — API 与通信、范围控制；`epics.md` Additional Requirements |
| **NFR-P2** | 过程可视更新 **批处理/节流**，不阻塞主流式；与现有 channel 流式协调 | `architecture.md` — 性能 NFR-P1–P3、实现模式（节流示例） |
| **单一真相源** | 过程事件权威在 **Rust**；GUI 仅经已文档化 **Tauri command / 事件** 消费（本故事完成「可订阅源」） | `architecture.md` — 运行时真相源、FR12–FR14 |
| **事件命名** | `snake_case`，过去式或名词短语；示例 `cortex_toggled`、`swarm_phase_changed` | `architecture.md` — 过程/蜂群事件名 |
| **推送 vs 轮询** | 若尚未在 1.x 前序 ADR 封死，须在本故事或同一实现 ADR 中与 **1.3** 一并 **二选一封冻** | `epics.md` 待决项；`architecture.md` Implementation Sequence |

### 与 UX 的衔接（非本故事交付）

- **UX-DR3**：过程反馈不挡输入、不挡流式；可折叠 —— 由 Epic 2 实现，本故事保证 **源端节流** 支撑该目标。  
- **UX-IMPL-1**：前端仅从 Tauri/gateway 取数 —— 本故事不修改 Vue。

### 禁止事项

- 在本故事中实现完整 **ProcessFeedbackStrip** 或神经系统 UI。  
- 引入未列入白名单的「万能」事件类型或任意 serde 结构体上屏。  
- 在皮层 **关** 时仍大量发射与蜂群过程等价的事件而不文档化（与 FR3/1.4 冲突）。

### Project Structure Notes

- 工作区根：以本机 `agent-diva` workspace 为准。  
- 规划产物：`d:\newspace\_bmad-output\planning-artifacts\`。

### Testing Requirements

- 至少 **一条** 无 GUI 自动化测试覆盖 **皮层 ON + 过程事件可见**（FR12 延伸）。  
- 节流行为：**推荐** 单元或集成级测试，非强制但若实现复杂则应有。

### Latest Tech Notes（2026-03-30）

- **Story 1.8** 将扩展 **终局/触顶** 白名单事件（如 `swarm_run_finished`、`swarm_run_capped`）与 `StopReason`；本故事 **v0 事件名** 应预留扩展空间但 **不必** 在本故事实现完整 ConvergencePolicy。  
- MSRV 与 clippy 规则沿用 `agent-diva/project-context.md`。

### References

- `epics.md` — Epic 1, Story 1.5；FR2；Additional Requirements（真相源、白名单、实现顺序）  
- `architecture.md` — NFR-P2、NFR-I2、运行时真相源、过程事件命名与载荷、ADR-E（终局事件为后续故事）  
- `prd.md` — FR2、API 版本化与过程事件订阅表述  
- `ux-design-specification.md` — UX-DR3、UX-IMPL-1（消费侧 Epic 2）  
- 前置：`1-2-cortex-state-persistence`、`1-3-gateway-swarm-sync-contract`、`1-4-cortex-off-headless-tests` 对应实现说明与测试

## Dev Agent Record

### Agent Model Used

Composer（Cursor 内联执行 `bmad-dev-story`）

### Debug Log References

- `process_events` 单测初版未 `flush_pending`，高频 phase 用例少计 2 条；已补 `flush_pending` 断言总数 12。
- `process_events_whitelist_reaches_sink` 在默认单线程 Tokio 下触发 MCP `block_in_place` 报错；测试改为 `multi_thread` runtime。

### Completion Notes List

- 在 `agent-diva-swarm` 新增 `process_events`：`ProcessEventV0` / `ProcessEventNameV0`（v0 白名单）、`ProcessEventPipeline`（`CortexRuntime` 门控 + 节流）、`ProcessEventBatchSink` 与 `ProcessEventRecorder` 探针。
- `AgentLoop` 增加可选 `with_process_event_pipeline`；在迭代开始发 `swarm_phase_changed`，工具起止发 `tool_call_started` / `tool_call_finished`；`Drop` 守卫在 turn 结束 `flush_pending`。
- 文档：`agent-diva-swarm/docs/process-events-v0.md`、`PROCESS_EVENTS_CORTEX_OFF.md`；根 `docs/swarm-cortex-contract-v0.md` 增加过程事件交叉引用；README 增补一节。
- 验证：`cargo test -p agent-diva-swarm -p agent-diva-agent`（含 `process_events` 过滤用例）通过。

### File List

- `agent-diva/agent-diva-swarm/src/process_events.rs`（新）
- `agent-diva/agent-diva-swarm/src/lib.rs`
- `agent-diva/agent-diva-swarm/README.md`
- `agent-diva/agent-diva-swarm/docs/process-events-v0.md`（新）
- `agent-diva/agent-diva-swarm/docs/PROCESS_EVENTS_CORTEX_OFF.md`（新）
- `agent-diva/agent-diva-agent/Cargo.toml`
- `agent-diva/agent-diva-agent/src/agent_loop.rs`
- `agent-diva/agent-diva-agent/src/agent_loop/loop_turn.rs`
- `docs/swarm-cortex-contract-v0.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/1-5-min-process-events-ui.md`

### Change Log

- 2026-03-30：实现 Story 1.5 最小过程事件发射、节流、皮层关语义文档与 AgentLoop 接线；无 GUI 单测覆盖白名单到达探针与节流批次。
- 2026-03-31：code review patch — `tool_call_started` 过程事件 message 经 `sanitize_tool_summary_for_process_event`；`ProcessEventPipeline` 去除重复 `enabled` 读取。

### Review Findings

- [x] [Review][Patch] `tool_call_started` 的 `message` 未经过 `sanitize_tool_summary_for_process_event`，与 `tool_call_finished` 及 NFR-S2/模块注释不一致，预览仍可能含控制字符或异常空白 [agent-diva/agent-diva-agent/src/agent_loop/loop_turn.rs:277-282] — 已修复（2026-03-31）
- [x] [Review][Patch] `ProcessEventPipeline::try_emit` 与 `flush_pending` 在已读取 `enabled` 并短路后仍重复调用 `cortex.snapshot().enabled`，冗余且略增误读成本 [agent-diva/agent-diva-swarm/src/process_events.rs:359-367,383-393] — 已修复（2026-03-31）
- [x] [Review][Defer] `ProcessEventBatchSink::deliver_batch` 若同步阻塞会拉长单次 turn；trait 与 `process-events-v0.md` 已约定下游须轻量/非阻塞，属集成方责任 — deferred, pre-existing [agent-diva/agent-diva-swarm/src/process_events.rs:256-258]

---

_Context: Ultimate BMad Method story context — 与 `1-1-swarm-crate-workspace.md` 模板对齐，2026-03-30 生成。_
