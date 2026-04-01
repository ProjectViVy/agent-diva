---
story_key: 2-1-cortex-toggle-ui
story_id: "2.1"
epic: 2
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/planning-artifacts/prd.md
  - agent-diva/agent-diva-gui/project-context.md
depends_on:
  - epic: 1
    story_id: "1.3"
    note: Gateway 与蜂群状态同步契约（Tauri command / DTO）须已落地，本 story 仅消费该契约。
---

# Story 2.1：聊天主页大脑皮层开关 UI（CortexToggle）

Status: done

## Story

作为一名**用户**，  
我希望**在聊天主页通过带大脑图标的开关控制蜂群层**，  
以便**能按任务深浅切换模式（FR1、FR4）**。

## Acceptance Criteria

1. **Given** 主聊天界面已加载  
   **When** 点击或键盘切换  
   **Then** 调用 **Epic 1 Story 1.3** 已定义的契约（Tauri `invoke` 或项目既定 HTTP 封装），且 UI **立即反映** 新状态（目标满足 **NFR-P1**：约 500ms 内完成状态与首帧反馈，不含模型/网络）

2. **And** 满足 **UX-DR1**：大脑皮层开/关在聊天主页 **一眼可辨**（**FR4**）；实现 **i18n**（用户可见文案走 `vue-i18n` key，与现有模式一致）、**ARIA** 角色/状态（开关为可访问切换控件）、**`:focus-visible`** 键盘焦点可见

3. **And** 符合 **NFR-A1**：皮层控件 **可命名、可键盘操作**（Tab 可达、Space/Enter 或平台惯例切换，与组件语义一致）

## 范围与放置

- **Vue：** 在 **`ChatView`**（或与其等价的聊天主页容器，以 `agent-diva-gui` 现有结构为准）中集成 **`CortexToggle`**（或等价命名的单文件组件），使用 **大脑图标** + 开关语义，与 **FR1**（启用/停用蜂群层）对齐。
- **本 story 不包含：** gateway 拒绝/超时时的显错与回滚（属 **Story 2.2**）；过程条/流式附属反馈（属 **Story 2.3** 等）。若调用 1.3 契约失败，可先采用 **最小诚实行为**（如不静默篡改本地「假状态」），完整错误 UX 在 2.2 闭合。

## Tasks / Subtasks

- [x] **依赖与契约核对**（AC: #1）  
  - [x] 确认 **Story 1.3** 已提供：查询当前皮层状态、切换状态的 **已文档化** 入口（command 名、载荷、serde 字段与 **版本字段** 对齐）  
  - [x] 在前端 `src/api/`（或项目既定封装）增加/对齐 **类型与调用**，禁止在 `.vue` 内手写与 Rust 不一致的 DTO（见 `architecture.md` 格式模式）

- [x] **实现 `CortexToggle` + 接入 `ChatView`**（AC: #1–#3, FR1, FR4）  
  - [x] 开关 **ON/OFF** 与后端真相源一致；**乐观 UI** 仅在与 1.3 契约约定一致时采用，且不得与 **NFR-P1** 感知目标冲突  
  - [x] 图标：使用项目现有图标库（如 `lucide-vue-next`）中选与「大脑/皮层」隐喻一致且 **自解释** 的图标  
  - [x] **i18n：** 新增 zh-CN（及与仓库默认语言策略一致的 en 等）文案 key；**禁止** 用户可见硬编码字符串散落

- [x] **可访问性**（AC: #2–#3, UX-DR1, NFR-A1）  
  - [x] `role="switch"` 或等价语义、`aria-checked` / `aria-label`（或 `aria-labelledby`）与可见标签一致  
  - [x] 全局样式或组件级 **`:focus-visible`** 轮廓，避免仅靠鼠标 hover 表达焦点  
  - [x] 键盘：焦点顺序合理，切换键行为可测

- [x] **验证**（AC: #1–#3）  
  - [x] `npm run build`（`vue-tsc --noEmit` + `vite build`）通过  
  - [x] 手动：聊天主页加载后切换开/关，状态与 gateway/后端一致（1.3 契约）；键盘路径可走通

## Dev Notes

### Epic 2 上下文

本 Epic 目标：用户在主聊天界面 **操作并理解** 大脑皮层开关，并在启用时看到过程反馈；轻量/触顶与用量提示与 UX-DR4、UX-DR5 等对齐。**本 story 仅交付开关 UI 与 1.3 契约接线**，不实现完整错误恢复（2.2）与过程条（2.3）。

### UX-DR1（摘录）

- 大脑皮层开/关须在聊天主页 **一眼可辨**（**FR4**）。  
- 体验原则：**契约先于皮肤** — 可见状态须来自 **后端或诚实 stub**（本 story 以 **1.3 真相源** 为准，前端不长期自建第二真相源）。  
- 参考：`ux-design-specification.md` — Effortless Interactions、UX Decisions Register **UX-DR1**。

### 架构合规（Frontend / Tauri）

| 主题 | 要求 | 来源 |
|------|------|------|
| 真相源 | 皮层状态以 **Rust/gateway** 为准；Vue **invoke / 订阅**，不复制编排状态为第二真相源 | `architecture.md` — Frontend Architecture、Critical Architectural Decisions |
| 桥接 | Tauri **2** `invoke`；新 command **先对照** `commands.rs` 既有命名与错误形态 | `architecture.md` — Communication Patterns、`agent-diva-gui/project-context.md` |
| 组件位置 | 与 `ChatView` / `NormalMode` 同层或 `components/` 下可检索命名；避免散落 | `architecture.md` — Structure Patterns |
| 性能 | 切换反馈目标 **NFR-P1**；过程类节流属后续 story | `architecture.md` — NFR-P1 |
| 错误文案 | 用户可见走 **i18n**，不把内部堆栈当 toast | `architecture.md` — Format Patterns |

### 与 `agent-diva-gui/project-context.md` 的关系

- 入口参考：`src/components/ChatView`（及 `App.vue` 布局）。  
- 修改 Tauri command 须同步 `src/api/*.ts`。  
- 技术栈：Vue 3、Vite 6、Tailwind 3、`vue-i18n`、MSRV 与质量闸与 workspace 一致。

### 禁止事项

- 在本 story 中实现 **Story 2.2** 的完整错误回滚产品文案与中间态治理（可留 TODO 与 2.2 对齐）。  
- 在 Vue 中 **直接实现** gateway 编排或绕过 1.3 契约写状态。  
- 跳过 **ARIA / i18n / focus-visible** 导致 **NFR-A1** 或 **UX-DR1** 不达标。

### Testing Requirements

- 以 **构建通过** + **手动可访问性抽检** 为主；若仓库已有 Vue 组件测试基础设施，可为 `CortexToggle` 补充 **角色与键盘** 冒烟测试（非强制，除非项目规范要求）。

### Latest Tech Notes（2026-03-30）

- GUI 路径：`agent-diva/agent-diva-gui/`（相对仓库根 `newspace`）。  
- **Story 1.3** 未就绪时：本 story **阻塞** 或仅能对接 **stub**（须在实现说明中标明，且不得验收为生产就绪）。

### References

- `epics.md` — Epic 2, Story 2.1；Epic 1 Story 1.3（依赖）  
- `ux-design-specification.md` — UX-DR1、旅程一/二（大脑皮层开关）  
- `architecture.md` — Frontend Architecture、Tauri、Naming/Format/State、NFR-P1、FR12–FR14  
- `prd.md` — FR1、FR4、FR13、FR14、NFR-A1、NFR-P1  
- `agent-diva/agent-diva-gui/project-context.md` — 模块边界、API 封装、构建命令

## Dev Agent Record

### Agent Model Used

Cursor 内联代理（Composer）

### Debug Log References

（无）

### Completion Notes List

- 新增 `src/api/cortex.ts`：`CortexState`（`enabled` / `schemaVersion`）与 `get_cortex_state`、`toggle_cortex`（及 `set_cortex_enabled`）封装，对齐 `docs/swarm-cortex-contract-v0.md`。
- 新增 `CortexToggle.vue`：`Brain` 图标 + 可见开/关文案；**invoke 成功后再更新 UI**（非乐观翻转）；`toggle` 失败时 `showAppToast` + i18n（`toggleSyncFailed` / `toggleSyncFailedCodeRejected`），并尝试 `get_cortex_state` 回同步；**首次拉取失败**时不冒充「已关闭」，展示加载/错误文案与 **重试**（`loadingState`、`loadFailedShort`、`retryLoad`）；订阅 `cortex_toggled`（收到有效 payload 时视为已就绪）；`role="switch"`、`aria-checked`、`aria-labelledby`、`focus-visible`；原生 `button` 支持 Space/Enter。
- `ChatView.vue`：Tauri 运行时内在消息区上方展示皮层工具条并挂载 `CortexToggle`。
- i18n：上述 `cortex.*` 键（zh、en）。
- 验证：`npm run build`（含 `vue-tsc --noEmit`）通过。

### File List

- `agent-diva/agent-diva-gui/src/api/cortex.ts`
- `agent-diva/agent-diva-gui/src/components/CortexToggle.vue`
- `agent-diva/agent-diva-gui/src/components/ChatView.vue`
- `agent-diva/agent-diva-gui/src/locales/en.ts`
- `agent-diva/agent-diva-gui/src/locales/zh.ts`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `_bmad-output/implementation-artifacts/2-1-cortex-toggle-ui.md`

### Change Log

- 2026-03-30：实现 Story 2.1 — CortexToggle + 1.3 契约接线、可访问性与 i18n；sprint 状态 → review。
- 2026-03-31：代码评审 patch — 首屏加载失败诚实态 + 重试；同步 Dev Agent Record；story / sprint → done。

### Review Findings

- [x] [Review][Patch] 首次 `get_cortex_state` 失败时仍在 `finally` 中将 `initialized` 置为 `true`，`enabled` 保持默认 `false`，开关呈现「已关闭」且可操作，可能与后端真实状态不一致，违背故事强调的「真相源」与 UX-DR1 诚实展示；建议首次拉取失败时保持禁用/未知态、提供重试或错误提示，或仅在成功拿到快照后再 `initialized = true`。 [agent-diva/agent-diva-gui/src/components/CortexToggle.vue:42-50] — **已修复（2026-03-31）**：`ready` / `loadError` / `loadingInitial` + 重试按钮与 `cortex_toggled` 恢复就绪。

- [x] [Review][Patch] Dev Agent Record 中 Completion Notes 写「调用失败仅 `console.error`、无产品级 toast」，与当前 `CortexToggle.vue` 中 `showAppToast` + `cortex.toggleSyncFailed` / `toggleSyncFailedCodeRejected` 不一致；请更新记录以免后续评审误判范围（实现本身与契约/2.2 预留可接受）。 [_bmad-output/implementation-artifacts/2-1-cortex-toggle-ui.md § Dev Agent Record] — **已修复（2026-03-31）**：已重写 Completion Notes。

---

_Context: Ultimate BMad Method story context — `bmad-create-story` 于 2026-03-30 生成。_
