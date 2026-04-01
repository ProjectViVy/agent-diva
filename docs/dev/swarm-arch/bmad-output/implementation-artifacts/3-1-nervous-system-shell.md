---
story_key: 3-1-nervous-system-shell
story_id: "3.1"
epic: 3
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - agent-diva/agent-diva-gui/src/components/NormalMode.vue
---

# Story 3.1：神经系统路由与壳页面（NervousSystemView）

Status: review

## Story

作为 **用户**，  
我希望 **从侧栏进入神经系统全屏视图**，  
以便 **不再只看到永久的「即将推出」空壳（FR5）**。

## Acceptance Criteria

1. **Given** 侧栏 `neuro`（或最终路由键名）入口  
   **When** 用户点击进入  
   **Then** 在主内容区渲染 **专用 Vue 壳组件**，占满可用区域，并提供 **返回聊天** 的明确导航  
   **And** i18n 与 **中控台** 入口在 **图标与文案 key 上可区分**（**UX-DR2**；侧栏现用 `Server` + `nav.console` 与 `Heart` + `nav.neuro`，须保持语义不混用）

2. **And** **首屏子视图** 必须为 **BrainOverview**：**左右两分区**、**架构图式** 呈现、两分区均有 **非空的产品语义标签**（**FR15**、**UX-IMPL-2**）；可选首入一行 hint；键盘左右切换分区为 **可选** 增强

3. **And** **不得** 将 **游戏化总控台** 或 **《头脑特工队》式多角色忙碌** 作为进入神经系统后的 **必经首屏**（**FR16**）；本 story **不** 交付中控台式 hub 或多角色动画主路径

## Tasks / Subtasks

- [x] **在 `agent-diva-gui` 中新增 `NervousSystemView` 壳**（AC: #1）  
  - [x] 布局：全屏主内容区与现有 **NormalMode** 全屏页 **外边距 / flex 结构** 对齐（UX：与 `NormalMode` 一致，不另起一套 page padding）  
  - [x] 顶栏或等价区域：**神经系统标题**（独立 i18n key，勿复用 `nav.console` 语义）、**返回聊天**（清空 `activeMenu`、回到 `activeTab === 'chat'` 或与现有 `navigateTo('chat')` 等价行为）  
  - [x] 预留插槽或子路由位：`BrainOverview`（本 story 内联或同目录组件）+ 为 **Story 3.2** 预留 `NeuroDetailPanel` 挂载点（可先空或占位不阻断 MVP）

- [x] **实现 `BrainOverview`（MVP 首屏）**（AC: #2）  
  - [x] 左右两区：**架构图式**（SVG/简化图形均可，须可读为「大脑分区」而非控制台仪表）  
  - [x] 每区 **非空标签**（产品语义，非「左/右」占位；中英 i18n）  
  - [x] 可选：首次进入一行 **hint**；可选：`select-hemisphere` 事件或等价为 3.2 铺路  
  - [x] **禁止**：首屏以多角色忙碌动画、游戏化总控台为主视觉（**FR16**）

- [x] **接线 `NormalMode.vue`**（AC: #1–#3）  
  - [x] 当 `activeMenu === 'neuro'` 时，**不再** 渲染通用「即将推出」块；改为渲染 `NervousSystemView`（内嵌 `BrainOverview`）  
  - [x] `activeMenu === 'console'`（中控台）行为 **保持与现有一致**（可仍为 coming soon 或既有实现），确保 **神经 vs 控制台** 双入口 **图标 + key** 分离（**UX-DR2**）  
  - [x] 若文件内存在作者保留注释：以 **产品/架构验收**（Epic 3、FR5）为准，用 **独立组件文件** 承载新 UI，尽量减少对注释区块的无关编辑；必要时在 PR 说明 **替换占位与规划一致**

- [x] **i18n**（AC: #1–#2）  
  - [x] 新增神经系统标题、返回、BrainOverview 分区标签、可选 hint；**不得** 与 `nav.console` 混用同一套「中控台」文案

- [x] **验证**（AC: #1–#3）  
  - [x] 手动：侧栏 neuro → 全屏壳 + BrainOverview → 返回聊天  
  - [x] 手动：侧栏 console 与 neuro **文案与图标** 可区分  
  - [x] （若仓库已有）补充最小 **组件测试或 E2E 冒烟**：进入神经后可见分区标签，非「即将推出」

## Dev Notes

### Epic 3 上下文

Epic 3 目标：替换 `NormalMode` 占位；**MVP 首屏** = **大脑架构图 + 左右分区**；真实数据或 **诚实 stub**；排障线索在 **3.3**；**FR16** 愿景占位隔离在 **3.4**。  
**本 story** 聚焦 **路由壳 + BrainOverview 首屏**，**不** 实现完整详情数据、空错闲模板（属 3.2–3.3）。

### 架构落点（NormalMode.vue）

| 主题 | 要求 | 来源 |
|------|------|------|
| 替换点 | 神经系统占位在 **`NormalMode.vue`**；全屏神经页与聊天/设置并列于 `main` | `architecture.md` — Project Structure、`NormalMode.vue` |
| 组件位置 | 神经相关组件与 `NormalMode` 同层或 `components/neuro/`（新建目录须在架构或 ADR 记录） | `architecture.md` |
| 数据 | 首屏可为 **静态或诚实 stub**；长期须经 Tauri / gateway，**禁止** 仅前端伪造皮层真相源 | `architecture.md`、UX 旅程四 |

### UX 决策摘录

| ID | 与本 story 关系 |
|----|-----------------|
| **UX-DR2** | 神经系统 vs 中控台 **术语、入口、图标、i18n key** 区分 |
| **UX-IMPL-2** | **BrainOverview（MVP）**：左右分区 + 架构图式；非空语义标签；可选首入 hint；键盘切换可选 |

### FR 对齐

- **FR15：** MVP 内进入神经系统后首屏须为 **DIVA 大脑架构图式主视图** + **左右可区分区域**（非空标签）；**不得** 以游戏化总控台或多角色忙碌为必经首屏。  
- **FR16：** 游戏化总控台优先、多角色忙碌 **非 MVP**；本 story **不** 将其设为主路径；若仅无占位，**不** 要求实现愿景卡片（留给 3.4）。

### 禁止事项

- 将 **中控台（console）** 与 **神经系统（neuro）** 混用同一套标题/图标/key。  
- 首屏做成 **游戏控制台** 或 **多代理剧场** 作为主体验。  
- 在壳内引入 **仅前端持久化** 的皮层状态（与 NFR/架构不符）。

### Project Structure Notes

- GUI 根：`d:\newspace\agent-diva\agent-diva-gui\`（以本机为准）。  
- 规划产物：`d:\newspace\_bmad-output\planning-artifacts\`。

### Testing Requirements

- 至少 **一条** 自动化或清单化手测：neuro 路径可见 **BrainOverview 双标签**，且 **返回聊天** 可用。  
- 详情列表、空错闲文案属 **Story 3.2–3.3**，本 story **不强制**。

### References

- `epics.md` — Epic 3、Story 3.1、Story 3.4（FR16 隔离）  
- `ux-design-specification.md` — UX-DR2、UX-IMPL-2、`NervousSystemView`、`BrainOverview`、旅程三  
- `prd.md` — FR5、FR15、FR16、神经系统 UI 分期  
- `architecture.md` — NormalMode 替换点、FR15–FR16、前端组件边界  
- `NormalMode.vue` — `activeMenu`、`navigateTo`、侧栏 `console` / `neuro` 分支

## Dev Agent Record

### Agent Model Used

Cursor Composer（bmad-dev-story 工作流）

### Debug Log References

### Completion Notes List

- 新增 `components/neuro/NervousSystemView.vue`：顶栏 `neuro.viewTitle` + `neuro.backToChat`（`navigateTo('chat')`），主区 `p-4` + `BrainOverview`，`#neuro-detail-panel-root` 供 3.2 挂载。
- 新增 `components/neuro/BrainOverview.vue`：左右分区 SVG（clip 强调半脑）、产品语义标签与描述、可关闭首入 hint（localStorage）、`select-hemisphere` 与全局 ←/→（输入框聚焦时不拦截）。
- `NormalMode.vue`：`activeMenu === 'neuro'` 单独分支渲染神经壳；`console` 仍走原「敬请期待」块与作者注释，未改动该块内部。
- i18n：`zh.ts` / `en.ts` 新增 `neuro.*`，与 `nav.console` / `nav.neuro` 分离；同批工作区还包含与其它 epic 并线的 `cortex.*`、`settings.capabilityManifest`（Code review 决议：视为同批交付，File List 已补全以免验收歧义）。
- `npm run build`（vue-tsc + vite）通过；`neuro` 与 `api/neuro` 配套含 Vitest 单测文件。

### File List

- `agent-diva/agent-diva-gui/src/components/neuro/NervousSystemView.vue`（新增）
- `agent-diva/agent-diva-gui/src/components/neuro/BrainOverview.vue`（新增）
- `agent-diva/agent-diva-gui/src/components/neuro/NeuroDetailPanel.vue`（新增）
- `agent-diva/agent-diva-gui/src/components/neuro/DataPhaseBadge.vue`（新增）
- `agent-diva/agent-diva-gui/src/components/neuro/NervousSystemView.spec.ts`（新增）
- `agent-diva/agent-diva-gui/src/components/neuro/NeuroDetailPanel.spec.ts`（新增）
- `agent-diva/agent-diva-gui/src/components/neuro/DataPhaseBadge.spec.ts`（新增）
- `agent-diva/agent-diva-gui/src/components/neuro/neuroVision.i18n.spec.ts`（新增）
- `agent-diva/agent-diva-gui/src/api/neuro.ts`（新增）
- `agent-diva/agent-diva-gui/src/api/neuro.spec.ts`（新增）
- `agent-diva/agent-diva-gui/src/components/NormalMode.vue`（修改）
- `agent-diva/agent-diva-gui/src/locales/zh.ts`（修改）
- `agent-diva/agent-diva-gui/src/locales/en.ts`（修改）

### Change Log

- 2026-03-30：实现神经系统壳页与 BrainOverview MVP，接线侧栏 neuro 入口；sprint `3-1-nervous-system-shell` → `review`。
- 2026-03-31：Code review 批量修复：`NeuroDetailPanel` 根节点 `id="neuro-detail-panel-root"`；`cortex_toggled` 监听 `try/catch`；i18n 并线范围记入 Completion Notes；File List 补全；`git add` neuro + `api/neuro`；story / sprint → `done`。

### Review Findings

（Code review：`3-1`，`full` 模式，spec = 本文件；子代理未单独起进程，由主会话按 Blind / Edge / Acceptance 三层口径合并结论。）

- [x] [Review][Decision] `zh.ts` / `en.ts` 中除 `neuro.*` 外还新增 `cortex`、`settings.capabilityManifest` 等大段文案 — **已决议**：视为与本分支同批交付（多 epic 并线），在 Completion Notes 与 File List 中显式记录，避免与「仅 neuro」叙述冲突。

- [x] [Review][Patch] `#neuro-detail-panel-root` — **已修复**：`NeuroDetailPanel.vue` 根 `aside` 增加 `id="neuro-detail-panel-root"`。

- [x] [Review][Patch] `listen('cortex_toggled')` — **已修复**：`NervousSystemView.vue` 的 `onMounted` 内 `try/catch`，失败时记录错误并依赖已完成的首次 `refreshSnapshot()`。

- [x] [Review][Patch] 未跟踪文件与 File List — **已处理**：`git add` `components/neuro/**`、`api/neuro.ts`、`api/neuro.spec.ts`；File List 已更新。

---

_Context: Ultimate BMad Method story context — 对齐模板 `1-1-swarm-crate-workspace.md`，`bmad-create-story` 于 2026-03-30 生成。_
