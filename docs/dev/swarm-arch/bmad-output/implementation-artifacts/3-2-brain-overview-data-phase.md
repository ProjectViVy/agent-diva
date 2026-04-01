---
story_key: 3-2-brain-overview-data-phase
story_id: "3.2"
epic: 3
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/planning-artifacts/prd.md
---

# Story 3.2：BrainOverview + 分区详情与数据阶段

Status: done

## 前置依赖（须先满足）

- **Story 3.1**：神经系统路由与壳页面；首屏 `BrainOverview`、左右分区与入口行为已就绪。  
- **Story 1.5**：最小过程事件在发射侧存在，且与 **gateway / 皮层契约** 一致。  
- **Gateway**：神经详情所消费的快照或流 **必须与** Tauri command / 既有 gateway 客户端 **同源**（对齐 **UX-IMPL-1**、**FR14**），**禁止** 与聊天区各自捏造两套「皮层真相」。

## Story

作为 **用户**，  
我希望 **点击 BrainOverview 左/右分区后，能看到连接或活动信息，或在数据未就绪时看到诚实说明**，  
以便 **理解系统内在在做什么（FR6）**，且 **不会误以为 stub 是实时真值**。

## Acceptance Criteria

1. **Given** 后端已提供 **快照 API**，或产品明确允许的 **stub 标志**（与 gateway 约定一致）  
   **When** 用户选中左分区或右分区  
   **Then** **`NeuroDetailPanel`** 展示 **列表式真实数据**，或展示 **带数据阶段说明的占位**（**UX-IMPL-4**）  
   **And** 占位场景下须 **`DataPhaseBadge` 或等价文案**，对 `live` / `stub` / `degraded` 等 **诚实标注**（**FR6**；见 `ux-design-specification.md` — `DataPhaseBadge`、`Additional Patterns` 中 Stub 数据要求）

2. **And** 展示内容 **与 Story 1.5 / gateway 同源**：若聊天区过程条或皮层状态已反映某阶段，神经详情 **不得** 与之矛盾（同一 DTO / 同一订阅出口优先）

3. **And** **`NeuroDetailPanel` 的 stub 须「诚实」**：  
   - **禁止** 用静态假列表冒充实时连接/活动；  
   - **允许** 明确文案 + 角标/徽章说明「演示数据」「未连接后端」「降级」等；  
   - **禁止** 骨架屏或动效 **暗示已有真实数据** 而实际为 stub（对齐 UX：**禁止** 骨架暗示已有数据若实际为 stub）

## Tasks / Subtasks

- [x] **冻结神经快照 / 详情的数据形状**（AC: #1–#2）  
  - [x] 与 **1.3 / 1.5** 已有 DTO 或事件字段对齐，避免神经专用第二套状态机  
  - [x] 在 command 或 store 层标注 **data phase**（`live` | `stub` | `degraded` 等，以实现为准，须可测）

- [x] **实现 `NeuroDetailPanel` 与分区选中联动**（AC: #1）  
  - [x] 接收 `BrainOverview` 的 `select-hemisphere`（或等价）与当前快照 props  
  - [x] **有数据**：渲染列表/行（状态点、简短标签，与 UX `NeuroDetailPanel` 描述一致）  
  - [x] **无数据或 stub**：展示占位 + **`DataPhaseBadge` 或等价 i18n 文案**（**UX-IMPL-4**）

- [x] **接线 gateway / Tauri**（AC: #2）  
  - [x] 仅通过 **Tauri command 或已有 gateway 客户端** 注入；不在组件内长期伪造皮层状态（**UX-IMPL-1**）

- [x] **验证**（AC: #1–#3）  
  - [x] 手工或自动化：**stub 模式下** 界面可见 **诚实标注**，且无「像真数据」的误导布局  
  - [x] 与聊天页同一会话下 **阶段/事件语义一致**（抽样对照 1.5 事件或快照）

## Dev Notes

### UX-IMPL-4（本故事硬约束）

- **Stub 诚实标注：** 任何非 `live` 数据 **必须** 配 `DataPhaseBadge` 或 **等价可见文案**（**FR6**）。  
- 参考：`ux-design-specification.md` — **Custom Components**（`NeuroDetailPanel`、`DataPhaseBadge`）、**Implementation Roadmap** Phase 2、`Feedback Patterns` 中「数据为 stub」类 **琥珀/信息** 提示、**Additional Patterns** — Stub 数据。

### 与 Epic 3 其它故事的边界

- **Story 3.3** 侧重 **空/错/闲模板化排障文案**（**UX-IMPL-3**、**FR7**）。本故事 **可先** 用最短占位 + 诚实角标满足 FR6；**不** 要求在本 story 内完成全部 3.3 模板。  
- **Story 3.4** 愿景占位不得阻断 MVP 路径；本故事 **不** 引入必经的游戏化总控台。

### 架构与 FR 映射

| 主题 | 要求 | 来源 |
|------|------|------|
| FR6 | 数据或 stub + **诚实阶段标注** | `epics.md` — FR Coverage、Story 3.2 |
| FR14 | 与 gateway **单一真相源** | `epics.md` — Epic 1 / 3 交叉 |
| UX-IMPL-1 | 数据仅经 Tauri / gateway | `epics.md` — UX-IMPL |
| UX-IMPL-4 | `DataPhaseBadge` 或等价 | `epics.md`、`ux-design-specification.md` |

## Dev Agent Record

### Agent Model Used

Cursor Composer（bmad-dev-story）

### Debug Log References

- 本地 `cargo test` 与默认 target 目录锁竞争时，使用独立 `CARGO_TARGET_DIR` 可完成 `agent-diva-swarm` 与 `agent-diva-gui` 集成测试。

### Completion Notes

- 在 `agent-diva-swarm` 新增 `neuro_overview`：`NeuroOverviewSnapshotV0`、`NeuroDataPhase`、`build_neuro_overview_snapshot_v0`，与 `CortexState` + `ProcessEventV0` 对齐；单元测试覆盖 degraded / stub / live 分区与 tool error 状态。
- Tauri `get_neuro_overview_snapshot` 与 `get_cortex_state` 共用 `Arc<CortexRuntime>`；过程事件缓冲尚未挂载时传入空切片 → 皮层开为 `stub`、关为 `degraded`，满足诚实标注（无假列表）。
- GUI：`NeuroDetailPanel`、`DataPhaseBadge`、`api/neuro.ts`；`NervousSystemView` 监听 `cortex_toggled` 刷新快照；浏览器预览使用 `previewNeuroOverviewSnapshot()` stub。
- Vitest：`neuro.spec.ts`、`DataPhaseBadge.spec.ts`、`NeuroDetailPanel.spec.ts`；保留 Story 3.1 的 `vision-stub` 折叠块以满足既有 `NervousSystemView.spec.ts`。

### Implementation Plan

1. Rust DTO + 可测 phase 推导 → 2. Tauri command → 3. Vue 接线 + i18n → 4. 自动化测试与 `vue-tsc`。

## File List

- `agent-diva/agent-diva-swarm/src/neuro_overview.rs`（新）
- `agent-diva/agent-diva-swarm/src/lib.rs`
- `agent-diva/agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva/agent-diva-gui/src-tauri/src/lib.rs`
- `agent-diva/agent-diva-gui/src-tauri/tests/neuro_overview_snapshot_contract.rs`（新）
- `agent-diva/agent-diva-gui/src/api/neuro.ts`（新）
- `agent-diva/agent-diva-gui/src/api/neuro.spec.ts`（新）
- `agent-diva/agent-diva-gui/src/components/neuro/DataPhaseBadge.vue`（新）
- `agent-diva/agent-diva-gui/src/components/neuro/DataPhaseBadge.spec.ts`（新）
- `agent-diva/agent-diva-gui/src/components/neuro/NeuroDetailPanel.vue`（新）
- `agent-diva/agent-diva-gui/src/components/neuro/NeuroDetailPanel.spec.ts`（新）
- `agent-diva/agent-diva-gui/src/components/neuro/NervousSystemView.vue`
- `agent-diva/agent-diva-gui/src/locales/en.ts`
- `agent-diva/agent-diva-gui/src/locales/zh.ts`

## Change Log

- **2026-03-31：** Story 3.2 — 神经总览快照 DTO、`get_neuro_overview_snapshot`、详情面板与数据阶段角标、测试与 sprint 状态 `review`。
- **2026-03-31：** Code review — 拍板选项 1：MVP 接受诚实 `stub` + `commands.rs` TODO，过程事件接入 `get_neuro_overview_snapshot` 顺延后续工作；`ToolCallFinished` → UI `status` 的契约已写在 `neuro_overview.rs` 注释。

### Review Findings

- [x] [Review][Decision] 桌面端神经快照是否必须在 Story 3.2 内接入过程事件源 — **已拍板（选项 1）**：本 story 以 MVP 验收；生产路径暂保持空事件切片与诚实 `stub`/`degraded`；与 1.5 同源的过程事件缓冲接入 `get_neuro_overview_snapshot` 作为后续 story / 技术债跟进（见 `commands.rs` TODO）。

- [x] [Review][Patch] 工具完成行的 error 状态依赖 message 子串 — **已处理**：在 `agent-diva-swarm/src/neuro_overview.rs` 的 `ToolCallFinished` 分支上方补充发射侧约定注释（失败时 `message` 宜含 `"error"`；未来 schema 可结构化）。
