---
story_key: 2-3-process-feedback-strip
story_id: "2.3"
epic: 2
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
---

# Story 2.3：过程反馈最小 UI（ProcessFeedbackStrip）

Status: done

## Story

作为一名 **用户**，  
我希望 **在大脑皮层开启时看到进行中的过程反馈**，  
以便 **我不必只盯着最终回复（FR2）**。

## 前置依赖

| 故事 | 角色 |
|------|------|
| **1.5** | 提供 **最小过程事件** 发射侧契约与数据源（本故事为 **消费侧** UI） |
| **1.8** | 提供 **终局 / 触顶** 事件与稳定 payload；**`lightweight` / `capped`** 态须与后端一致（UX-DR4） |
| **2.1** | **CortexToggle** 已落地；仅当皮层 **开** 时展示过程条（与 Epic 2 目标一致） |

## Acceptance Criteria

1. **Given** 大脑皮层 **开** 且后端发出 **Story 1.5** 约定之过程事件  
   **When** 任务进行中  
   **Then** 展示 **至少一种** 过程反馈（时间线、步骤条等 **择一**），且 **可折叠**（UX-DR3）

2. **And** **不遮挡** 主输入与主对话 **流式区**；过程更新须 **批处理 / 节流 / 合并**，**不得** 以高频 DOM 或同步重绘 **阻塞** 主流式 token 渲染（**UX-DR3、NFR-P2** —— **无「挡流」**）

3. **And** 可选用 **UX-IMPL-5** 视觉 token（如 `neuro-*`、`process-muted`）使过程条视觉上 **附属、弱对比**，不抢主 CTA

4. **And** 订阅 **Story 1.8** 的终局 / 触顶事件：条带须能呈现 **`lightweight`**（极简阶段 / 不展开全阶段链）与 **`capped`**（触顶 / 停止）态（UX-DR4）。**澄清（Code Review 2026-03-31）：** **`capped`** 与后端事件 `swarm_run_capped` 及载荷字段 **严格一致**；**`lightweight`** 当前 **无独立后端 wire**，前端以「当前 turn **无** `tool_call_started` / `tool_call_finished` 且非 `capped`」**启发式** 呈现单行摘要；若 1.8 后续增加显式 `lightweight`（或等价）信号，GUI 再对齐 wire。

5. **And** 当进入 **`capped`** 或等价停止态时，**主对话区** 须同时存在 **可读系统说明**（非仅控制台日志），与 UX 规格 **§2.5 D** 一致

## Tasks / Subtasks

- [x] **组件与目录**（AC: #1–#3）  
  - [x] 在 `agent-diva-gui` 中实现 **`ProcessFeedbackStrip`**（或 UX 中 `ProcessFeedback` 等价命名，**本故事内冻结**）  
  - [x] 建议路径：`src/components/swarm/`（与架构 / UX **Component Implementation Strategy** 一致）  
  - [x] 数据经 **Tauri 事件 / invoke** 或既有 gateway 客户端注入；**禁止** 在组件内伪造长期编排状态为第二真相源

- [x] **折叠与布局**（AC: #1、#2）  
  - [x] 提供 **展开 / 折叠**；折叠控件为 **次要行动**（描边 / 文本按钮），不抢夺发送焦点  
  - [x] 条带位置与 z-index 保证 **输入框与流式消息区始终可操作、可见**；小屏走查 **不增高输入区到不可用**（UX / HTML 线框 + NFR-P2）

- [x] **节流与「不挡流」**（AC: #2）  
  - [x] 实现 **事件合并 / `requestAnimationFrame` 批处理 / 定时间隔刷新** 等 **择一或组合**，并在 PR 或 Dev Notes 中 **写明策略**（NFR-P2 要求）  
  - [x] **禁止** 将过程事件插入 **与助手最终回复同一条流式通道** 造成解析混淆；过程与 **聊天流式** **分轨**（UX-DR3：过程反馈 **不挡流式**）

- [x] **状态机：idle / streaming / throttled / done / capped / lightweight**（AC: #4、#5）  
  - [x] 与 **1.8** DTO / 事件对齐 **`lightweight`、`capped`（或规范中的 `stopped`）**  
  - [x] **`capped`：** 条带显式停止态 + **主对话区系统消息**（可读说明 + 可选建议：关皮层 / 简化重试）  
  - [x] **`lightweight`：** 极简一行或单阶段，**不** 默认展开完整多阶段时间线（FR19–FR20、UX-DR4）

- [x] **样式与 a11y**（AC: #3）  
  - [x] 可选：`tailwind.config` / CSS 变量扩展 **`process-muted`**、`neuro-*`（UX-IMPL-5）  
  - [x] **live region** 若使用：仅 **关键阶段变化** 播报，避免每条事件刷屏（与 NFR-P2 一致；见 UX 规格无障碍段）

- [x] **验证**（AC: #1–#5）  
  - [x] 组件级测试：折叠、状态切换、**mock 1.5 / 1.8 事件**  
  - [x] 手动或 E2E：**皮层开** 时有过程条；**皮层关** 时不展示或 idle；触顶时 **对话区** 有说明（**E2E 未加**：依赖 Tauri+网关；验收以组件测 + 后端贯通测为准）

## Dev Notes

### Epic 2 上下文

本 Epic 目标：用户在主聊天界面 **操作并理解** 大脑皮层开关，并在开启时看到 **过程反馈**；**轻量 / 触顶** 与用量提示符合 **UX-DR4、UX-DR5**；满足 **NFR-P1、P2、A1** 与 **UX-DR1、UX-DR3**。  
**本故事不交付** `RunTelemetryHint` / FR22 完整挂点（属 **Story 2.4**）。

### UX 对齐摘要

| 编号 | 要求 |
|------|------|
| **UX-DR3** | 过程反馈 **不挡输入、不挡流式**；**可折叠**；**节流 / 合并**（NFR-P2） |
| **UX-DR4** | 轻量路径须传达 **会收敛**；**终态**可感知；**禁止** 仅靠无尽阶段动画；**`capped`** 与后端 **一致**；**`lightweight`** 见 AC #4 澄清（启发式至有 wire 为止） |
| **UX-IMPL-5** | 视觉 token：`neuro-*`、`process-muted` 等 **可选用** |

### 架构与 NFR-P2

| 主题 | 要求 | 来源 |
|------|------|------|
| **NFR-P2** | 过程可视更新 **不应** 单独拖垮主聊天流式；**批处理或节流策略须在实现中说明** | `architecture.md` Frontend Architecture / `prd.md` |
| **单一真相源** | 前端 **订阅 / 拉取** 服务端状态；避免 Vue 内复制编排状态 | `architecture.md` |

### 禁止事项

- 在本故事中实现 **神经系统全屏视图**（Epic 3）或 **RunTelemetryHint** 主流程（Story 2.4）。  
- **阻塞** 主通道流式解析或 **将过程文本混入** 助手最终回复流。  
- 无 **1.8** 对齐的 **`capped`** 语义时自行发明 UI 态名，导致与后端无法对账；**`lightweight`** 在 AC #4 已冻结为当前启发式，直至后端提供显式信号。

### Project Structure Notes

- GUI 工作区以仓库 **`agent-diva-gui`** 为准。  
- 规划产物：`_bmad-output/planning-artifacts/`。

### Testing Requirements

- 至少 **1** 条组件测试覆盖 **折叠 + 一态切换**（如 streaming → capped）。  
- 与仓库现状对齐可选 **Playwright** 冒烟：**皮层开 → 可见条带 → 触顶 → 对话区系统文案**。

### References

- `epics.md` — Epic 2, Story 2.3  
- `ux-design-specification.md` — `ProcessFeedbackStrip`、UX-DR3 / UX-DR4、UX-IMPL-5、§2.5 D、Component Roadmap Phase 1  
- `architecture.md` — Frontend Architecture（NFR-P2）  
- `implementation-artifacts/1-5-min-process-events-ui.md` — 过程事件发射侧边界  
- `implementation-artifacts/1-8-convergence-policy-fr20.md` — 终局 / 触顶事件与 GUI 约定  

## Dev Agent Record

### Agent Model Used

Cursor / Composer（`bmad-dev-story` 2-3）

### Implementation Plan（NFR-P2 双轨节流说明）

- **Rust / Gateway：** 继续沿用 `ProcessEventPipeline` 默认策略（`swarm_phase_changed` 100ms 或 32 条合并等，见 `process-events-v0.md`）；新增 `AgentEvent::SwarmProcessBatch` 经 bus → SSE 事件名 **`swarm_process`**，与 `delta` / `final` **分轨**。
- **Tauri：** `send_message` 将 `swarm_process` 转为前端事件 **`swarm-process-batch`**（带 `request_id`），不写入 token 流解析路径。
- **Vue `ProcessFeedbackStrip`：** 对 `events` prop 使用 **`requestAnimationFrame` 合并**刷新展示副本（`immediate: true` 的 watch），降低同一帧内多次更新的 DOM 压力；`aria-live="polite"` 仅在阶段/触顶类事件时给文案，避免逐条刷屏。

### Debug Log References

- Vitest：`ProcessFeedbackStrip` 初版未 `watch immediate`，条带未挂载；已修复。
- PowerShell：`cd /d` 无效，改用 `Set-Location` 跑 `cargo test`。

### Completion Notes List

- 贯通路径：`agent-diva-agent` `BusSwarmProcessSink` + `with_swarm_process_bus_publishing` → `agent-diva-manager` `build_agent_loop` 接线 → `handlers` SSE `swarm_process` → Tauri `swarm-process-batch` → `App.vue` 聚合 → `ChatView` / `ProcessFeedbackStrip`。
- **`capped`：** 解析 `swarm_run_capped` 时在 transcript 插入 **系统消息**（`chat-system-message`）；条带标题为「已触顶」。
- **`lightweight`（UX）：** 当前 turn **无** `tool_call_*` 且非 capped 时，用 **单行摘要** 而非完整时间线列表（启发式，与 FR19 轻量路径常见形态一致；无单独 wire 事件名）。
- 皮层 **关**：`ProcessFeedbackStrip` `:show="cortexLayerOn"` 隐藏；后端 `ProcessEventPipeline` 仍随 gateway 内 `CortexRuntime` 门控（与桌面 Tauri 皮层 **可能暂时不同步**，已知架构债，Story 2.2/后续 gateway 同步可收敛）。

### File List

- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/2-3-process-feedback-strip.md`
- `agent-diva/agent-diva-core/src/bus/events.rs`
- `agent-diva/agent-diva-agent/src/lib.rs`
- `agent-diva/agent-diva-agent/src/swarm_process_bus.rs`
- `agent-diva/agent-diva-agent/src/agent_loop.rs`
- `agent-diva/agent-diva-agent/src/agent_loop/loop_turn.rs`
- `agent-diva/agent-diva-manager/Cargo.toml`
- `agent-diva/agent-diva-manager/src/runtime.rs`
- `agent-diva/agent-diva-manager/src/handlers.rs`
- `agent-diva/agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva/agent-diva-gui/src/types/swarmProcess.ts`
- `agent-diva/agent-diva-gui/src/components/swarm/ProcessFeedbackStrip.vue`
- `agent-diva/agent-diva-gui/src/components/swarm/ProcessFeedbackStrip.spec.ts`
- `agent-diva/agent-diva-gui/src/components/ChatView.vue`
- `agent-diva/agent-diva-gui/src/components/NormalMode.vue`
- `agent-diva/agent-diva-gui/src/App.vue`
- `agent-diva/agent-diva-gui/src/locales/zh.ts`
- `agent-diva/agent-diva-gui/src/locales/en.ts`
- `agent-diva/agent-diva-gui/tailwind.config.js`

### Change Log

- **2026-03-31：** Story 2.3 — 过程条 UI、SSE/Tauri 分轨、`swarm_run_capped` 系统说明、组件测与 `cargo test` / `vue-tsc` 通过。
- **2026-03-31：** Code review 收尾 — AC #4 `lightweight` 启发式正式写入验收；`App.vue` 多批 capped 系统消息去重 + `swarm-process-batch` 单批单次合并更新。

### Review Findings

- [x] [Review][Decision] **`lightweight` 语义与 AC #4 的严格对齐** — **已解决：** 选项 1（接受启发式）；已更新 AC #4、UX-DR4 表与禁止事项表述。

- [x] [Review][Patch] **多批 `swarm_process` 可能重复插入 capped 系统消息** [`App.vue`] — **已修复：** `cappedSystemNoticePushedForRequestId` 按 `streamRequestId` 去重；新会话 / `clearMessages` 时重置。

- [x] [Review][Patch] **`swarm-process-batch` 内逐条 `push` 触发多次响应式更新** [`App.vue`] — **已修复：** 先解析整批再 `swarmProcessEvents.value = [...prev, ...parsed]` 单次赋值。

- [x] [Review][Defer] **皮层 UI 与 gateway 过程门控可能不同步** [`2-3-process-feedback-strip.md` Completion Notes] — 已记在 Dev Agent Record；属既有架构债，待 Story 2.2/后续 gateway 同步收敛。 — deferred, pre-existing

---

_Context: Ultimate BMad Method story context — 与 `1-1-swarm-crate-workspace.md` 模板对齐，`bmad-create-story` 于 2026-03-30 生成。_
