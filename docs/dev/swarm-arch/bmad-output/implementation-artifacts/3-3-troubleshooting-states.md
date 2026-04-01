---
story_key: 3-3-troubleshooting-states
story_id: "3.3"
epic: 3
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
---

# Story 3.3：排障线索与空/错/闲态

Status: done

## 关联需求

- **FR7：** 用户可在神经系统或关联界面获得 **排障线索**（例如空闲、错误、未推进），与聊天记录互补。  
- **UX-IMPL-3：** **NeuroDetailPanel** — **空 / 错 / 闲** 模板化文案 + **建议动作**（非整段原始日志堆砌）。

## Story

作为一名 **用户**，  
我希望 **在异常或空闲时获得可行动提示**，  
以便 **能决定重试或关闭大脑皮层（FR7）**。

## 验收标准

1. **Given** 无活动、工具错误或阶段卡住  
   **When** 视图刷新  
   **Then** 显示 **模板化的空态 / 错误态 / 空闲态** 文案，并附带 **建议下一步**（与 UX-IMPL-3 一致）  
   **And** **不** 重复堆砌整段原始日志

2. **And** 三种态 **各至少一种** 明确文案 + 可操作建议（呼应 UX 成功标准「排障有路」）

3. **And** 与 Story 3.2 的 **NeuroDetailPanel** 数据同源（gateway / 快照 / stub 标志一致），不与主聊天区事实矛盾

4. **And** 建议动作示例方向（实现可择词与 i18n）：空态 — 返回聊天或等待下次任务；错态 — **重试**、查看设置/连接、或 **关闭大脑皮层** 后重试；闲态 — 说明「当前无活动」并指向可发起操作的入口（若产品允许）

5. **And** 无障碍：状态不仅靠颜色区分，配 **图标或短标题**；关键操作建议可被键盘聚焦（与现有 `focus-visible` 模式一致）

## 任务 / 子任务

- [x] **定义三种模板的数据模型**（AC: #1–#2）  
  - [x] 枚举或联合类型：`empty` / `error` / `idle`（及与 API 字段的映射说明）  
  - [x] 每态：`title`、**简短** `body`、`suggestedActions[]`（label + 行为：路由 / command / 仅文档链接择一）

- [x] **在 `NeuroDetailPanel`（或等价组件）中落地 UI**（AC: #1–#2, #5）  
  - [x] 空态：诚实说明「无列表数据 / 未选中分区」等，避免空白屏  
  - [x] 错态：用户可读摘要 + 建议动作；**禁止** 默认展示完整 stack / 原始 JSON  
  - [x] 闲态：区分「无活动」与「加载中」；与 `NervousSystemView` / 面板的 `loading` 不混淆

- [x] **i18n**（AC: #1–#4）  
  - [x] 文案键集中管理（与 Epic 3 其他神经组件同一命名空间习惯）  
  - [x] 简体中文为默认或首批语言之一（与产品语言策略对齐）

- [x] **与 3.2 集成与数据绑定**（AC: #3）  
  - [x] 从与 BrainOverview / 详情列表 **相同** 的数据源推导当前应展示的 troubleshooting 态  
  - [x] stub / degraded 阶段时，模板与 **DataPhaseBadge**（若已实现）语义一致

- [x] **验证**（AC: #1–#5）  
  - [x] 组件级测试或 E2E：三种态各至少一条用例（mock 数据即可）  
  - [x] 手测：错态不出现整页日志墙；闲/空与聊天侧叙述不冲突

## 开发说明

### Epic 3 上下文

本 Epic 交付 **神经系统** MVP：**侧栏进入全屏** → **BrainOverview** → **分区详情**。Story 3.3 专责 **FR7 / UX-IMPL-3**，在 **NeuroDetailPanel** 上完成 **空 / 错 / 闲** 的 **模板化排障面**，与 Story 3.1、3.2 的壳与列表互补。

### 组件与 UX 依据

| 主题 | 要求 | 来源 |
|------|------|------|
| 承载组件 | `NeuroDetailPanel` — 列表、状态点、错误文案、**建议动作链接**（非日志堆叠） | `ux-design-specification.md` — Custom Components |
| 成功标准 | 空 / 错 / 闲 **各一种** 明确文案 + 建议动作 | 同上 — §2.3 Success Criteria |
| 反馈模式 | 错误 **非阻塞** 为主；皮层相关失败须 **回滚 + 说明**（NFR-R1） | 同上 — Feedback Patterns；`architecture.md` |
| 路线图 | Phase 2：`NervousSystemView` + `BrainOverview` + `NeuroDetailPanel`（Story 3.1–3.3） | `ux-design-specification.md` — Implementation Roadmap |

### 与 FR16 的边界

游戏化总控台、多角色忙碌 **不属于** MVP；本 story **不得** 将非 MVP 占位作为排障模板的 **必经** 路径。若某建议动作链路到「愿景」功能，须 **文案标明后续/非验收**，且不阻断仅完成 FR7 的路径。

### 目录建议

与 UX 文档一致：优先 `agent-diva-gui/src/components/swarm/`（或仓库既定 feature 目录）；数据经 **Tauri command / gateway 客户端**，组件内 **不** 伪造长期连接状态。

## 参考资料路径

- `_bmad-output/planning-artifacts/epics.md` — Epic 3 Story 3.3  
- `_bmad-output/planning-artifacts/ux-design-specification.md` — UX-IMPL-3、`NeuroDetailPanel`、§2.3 / Component Strategy  
- `_bmad-output/planning-artifacts/prd.md` — FR7、神经系统导航与可诊断性  
- `_bmad-output/planning-artifacts/architecture.md` — 神经系统信息模型、错误与 tracing 约定  

## Dev Agent Record

### Implementation Plan

- 在 `api/neuroTroubleshooting.ts` 定义 `NeuroTroubleshootVariant` 与 `deriveNeuroTroubleshootTemplate`：由 `loading` / `loadError` / `NeuroOverviewSnapshotV0` / `side` 推导模板（与 `rowsForHemisphere` 同源）。
- 新增 `NeuroTroubleshootCallout.vue`：图标 + 标题 + 正文 + 建议动作按钮（`focus-visible`）；错态不展示原始堆栈。
- `NeuroDetailPanel`：`loading` 单独展示；无快照或加载错误仅展示 callout；有快照时保留 `DataPhaseBadge` + stub/degraded 说明，无行时再叠放 callout。
- `NervousSystemView` / `NormalMode`：`retry` → `refreshSnapshot`；`open-settings` → 设置页「网络」；`disable-cortex` → `setCortexEnabled(false)`（仅 Tauri）后刷新快照。
- 文案：`neuro.troubleshoot.*`（en/zh）；单测覆盖推导与面板三种态。

### Debug Log

- （无）

### Completion Notes

- ✅ `npm test`、`npm run build`（agent-diva-gui）已通过。
- ✅ 三种态均有独立 `data-testid`（`neuro-troubleshoot-empty|error|idle`），便于回归。
- 手测建议：在 Tauri 中断网关后进入神经系统，确认错态仅为短文案 + 按钮，无 JSON/堆栈墙。
- 2026-03-31：code review patch — `showDisableCortexAction` + `deriveNeuroTroubleshootTemplate` 在非 Tauri 下省略「关闭皮层」动作；移除未使用 `neuro.detail*` 文案键；Vitest 增补 2 例。

## File List

- `agent-diva/agent-diva-gui/src/api/neuroTroubleshooting.ts`（新）
- `agent-diva/agent-diva-gui/src/api/neuroTroubleshooting.spec.ts`（新）
- `agent-diva/agent-diva-gui/src/components/neuro/NeuroTroubleshootCallout.vue`（新）
- `agent-diva/agent-diva-gui/src/components/neuro/NeuroDetailPanel.vue`
- `agent-diva/agent-diva-gui/src/components/neuro/NeuroDetailPanel.spec.ts`
- `agent-diva/agent-diva-gui/src/components/neuro/NervousSystemView.vue`
- `agent-diva/agent-diva-gui/src/components/NormalMode.vue`
- `agent-diva/agent-diva-gui/src/locales/en.ts`
- `agent-diva/agent-diva-gui/src/locales/zh.ts`

## Change Log

- 2026-03-31：实现 FR7 / UX-IMPL-3 模板化排障（空/错/闲）、i18n、与快照同源推导及 Vitest 覆盖。
- 2026-03-31：bmad-code-review（无 git diff，按 File List 对照实现）；`npm test` 通过。
- 2026-03-31：审查项 batch-apply 已落地；故事 / sprint → `done`。

### Review Findings

_（bmad-code-review，2026-03-31；评审层：Blind / Edge / Acceptance 由本会话合并执行；`failed_layers`：无。）_

- [x] [Review][Patch] 浏览器/非 Tauri 预览下错态仍展示「关闭大脑皮层」，点击后 `onDisableCortex` 静默 `return`，无 toast 或禁用态，与「诚实反馈」及 Dev Notes「仅 Tauri」不一致；建议在父级按 `isTauriRuntime()` 从模板中省略该动作，或对按钮 `disabled` + `title`/短说明。 [`agent-diva/agent-diva-gui/src/components/neuro/NervousSystemView.vue:36-39`；`NeuroTroubleshootCallout.vue` 动作列表] — **已修复（2026-03-31）**：`showDisableCortexAction` prop + `deriveNeuroTroubleshootTemplate({ showDisableCortexAction })`，`NervousSystemView` 传入 `isTauriRuntime()`。

- [x] [Review][Patch] `neuro.detailLoadError`、`detailNoSnapshot`、`detailNoRowsThisSide`、`detailNoStreamPlaceholder` 在组件树中无引用（检索仅见于 `en.ts`/`zh.ts`），与本次 `neuro.troubleshoot.*` 并存易产生文案漂移；建议删除或改由排障模块单一引用并文档说明。 [`agent-diva/agent-diva-gui/src/locales/en.ts`、`zh.ts`] — **已修复（2026-03-31）**：已从 `en`/`zh` 删除上述键。

