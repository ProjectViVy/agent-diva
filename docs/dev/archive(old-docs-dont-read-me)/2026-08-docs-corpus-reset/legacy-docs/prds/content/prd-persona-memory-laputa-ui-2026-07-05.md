---
title: "Persona & Memory — Laputa GUI Management Page"
status: draft
created: 2026-07-05
updated: 2026-07-05
version: 0.1.0
project: agent-diva-pro
author: John (PM)
related_prds:
  - docs/prds/prd-laputa-2026-06-12/prd.md
  - docs/prds/prd-selfinprove-2026-06-12/prd.md
---

# PRD: 人格与记忆 — Laputa GUI 管理页面

> **状态**: draft（ Coaching path 已收敛需求，待实现方 review 后转 final）
> **背景**: agent-diva-pro 分支中，Laputa 作为持续的人格与记忆系统已经落地，但 GUI 缺乏一个独立的查看/编辑入口。本 PRD 规划在“功能”板块新增“人格与记忆”页面。

## 0. Document Purpose

为 agent-diva-gui 定义一个新的“人格与记忆”页面，使用户能够：
1. 查看 Laputa 14 个 section 的分组列表与初始化状态；
2. 选择任一 section，查看并编辑其 Markdown 内容；
3. 保存修改时通过 Laputa 治理接口写入，并生成可审计的 changelog；
4. 查看单个 section 的变更历史（遮罩弹窗，可复制历史内容）。

本 PRD 是 `prd-laputa-2026-06-12` 的 GUI 消费方补充，不改动 Laputa 核心治理模型。

## 1. Vision

在 GUI“功能”板块最上方新增“人格与记忆”入口，作为 Diva 长期人格与记忆的“控制面板”。

- 它**不是** EvolutionView 的提案审批流（提案系统是上层）；
- 它**不是** Notebook 的报告系统；
- 它是 Laputa 持续记忆系统的直接视图：用户可查看当前记忆内容，并在项目早期直接编辑全部 section。

## 2. Target User

- 使用 agent-diva GUI 的 **power user / 开发者 / 实验者**；
- 需要直接干预 Diva 长期记忆、人格设定、关系描述、用户偏好的用户；
- 不接受仅通过提案系统间接修改，想要“打开即改”的早期体验。

## 3. Concerns

| Concern | 级别 | 说明 |
|--------|------|------|
| 写入必须可审计 | P0 | 即使是直接编辑，也必须走 Laputa 治理接口，生成 ChangelogRecord + AuditEvent，不可绕过。 |
| 与 EvolutionView 边界清晰 | P0 | 本页不处理 proposal 审批；保存动作可内部合成 proposal 并 apply，但 UI 不展示提案生命周期。 |
| 早期权限简化 | P1 | 当前阶段全部 section 可编辑；权限矩阵（agent_self / user_only / shared） deferred 到后续。 |
| 回滚与 diff | P2 | 本期不做回滚按钮和保存前 diff；仅提供历史弹窗可复制内容。 |
| 性能 | P2 | section 内容可能较大，需避免不必要的全量重渲染。 |

## 4. Scope Boundaries

### 4.1 In Scope
- “功能”板块新增“人格与记忆”导航项，置于最上方；
- 14 个 Laputa section 按分组展示；
- 左侧分组列表 + 右侧 Markdown 编辑器布局；
- section 内容查看与保存；
- 单个 section 的 changelog 历史弹窗（可复制历史内容）；
- 暴露/新增必要的后端 API（Tauri command + manager endpoint）及前端 `desktop.ts` 封装；
- 中英文 i18n 键值。

### 4.2 Out of Scope
- 提案审批工作流（保留在 EvolutionView）；
- 回滚操作按钮；
- 保存前 diff 预览；
- 系统级 AuditSink 日志（本页只消费 Laputa changelog）；
- section schema 校验与内容模板；
- 冲突检测/三向合并 UI（后端保留现有能力，前端本期不展示）。

### 4.3 Non-Goals
- 不替换 `EvolutionView`；
- 不引入新的持久化存储；
- 不做 Notebook 报告生成；
- 不实现用户权限分级（本期全部可编辑）。

## 5. Features & Functional Requirements

### FR-101 — 导航与页面入口
在 GUI 侧边栏“功能”板块最上方新增“人格与记忆”菜单项。

- 菜单键：`nav.personaMemory`；中文显示“人格与记忆”；英文显示“Persona & Memory”。
- 路由/激活键：`activeMenu = 'persona-memory'`。
- 在 `NormalMode.vue` 中新增 `v-else-if="activeMenu === 'persona-memory'"` 分支，挂载新组件 `PersonaMemoryView.vue`。
- 置于“功能”板块第一项（在“神经系统”之前）。

**验收**: 点击菜单后页面切换，URL/状态不报错，菜单高亮正确。

### FR-102 — Section 分组列表
页面左侧展示 Laputa 14 个 section 的分组列表。

分组定义：
| 分组 | section |
|------|---------|
| 人格 (Persona) | identity, relationship, commitment, preferences |
| 记忆 (Memory) | memory_md, history_md |
| 周期 (Periodic) | daily, weekly, monthly |
| 索引 (Indexes) | journal_reflective, proposal_inbox, changelog, report_indexes, aaak_summaries |

- 每个分组可折叠/展开；默认全部展开。
- 每个 section 项显示：中文名、英文名、初始化状态标签（已就绪 / 待定）。
- 状态依据 `LaputaSnapshot.sections[name].status`（`owned` / `tbd`）。
- 点击 section 项，右侧加载该 section 内容。
- 首次进入页面默认选中 `identity`。

**验收**: 14 个 section 全部可见，分组正确，TBD section 显示“待定”标签。

### FR-103 — Section 内容查看
右侧顶部显示当前 section 名称与最后更新时间，下方显示 Markdown 内容。

- 调用 `getLaputaSnapshot()` 获取全量列表与元数据；
- 调用 `getLaputaSection(name)` 获取当前 section 的 `content`；
- 内容以 Markdown 原文展示在编辑器中；
- 若 `.laputa/` 未初始化或 section 不存在，显示空状态文案并引导用户保存以初始化。

**验收**: 切换 section 后 1 秒内加载内容；未初始化状态不报错，显示引导。

### FR-104 — Section 内容编辑与保存
提供 Markdown 编辑器，用户修改后点击“保存”。

- 编辑器使用项目现有 Markdown 编辑组件（或纯 textarea），支持基本滚动与光标；
- “保存”按钮在内容变更后启用；
- 保存时调用 `writeLaputaSection(name, content, summary?)`：
  - 后端合成一个 `EvolutionProposal`（`proposal_type = memory_patch`），目标 section 为当前 section；
  - 调用 `LaputaService::apply_proposal` 完成写入、changelog、audit；
  - 返回 `ChangelogRecord`。
- 保存成功后：
  - 刷新当前 section 内容；
  - 刷新左侧最后更新时间；
  - 显示成功提示。
- 保存失败时显示错误文案（保留后端错误信息）。

**验收**: 修改 `commitment` 内容并保存后，`.laputa/` 对应文件更新，changelog 新增一条 apply 记录，GUI 显示成功提示。

### FR-105 — 变更历史弹窗（可复制）
每个 section 提供“历史”按钮，点击后弹出遮罩层/模态框，展示该 section 的 changelog 列表。

- 调用 `listLaputaChangelog({ target_section: name, page_size: 50 })`；
- 每条历史显示：时间、动作（apply / revert / rollback，本期多为 apply）、操作者、内容摘要；
- 提供“复制该版本内容”按钮，将对应 `after` 内容写入剪贴板；
- 弹窗可关闭（点击遮罩或关闭按钮）。

**验收**: 保存一次后打开历史弹窗，能看到新增记录；点击复制按钮后剪贴板内容与该版本一致。

### FR-106 — 后端 API 与前端封装
补齐/新增以下接口，确保 GUI 可调用。

| 接口 | 类型 | 说明 |
|------|------|------|
| `getLaputaSnapshot()` | Tauri command + manager GET `/api/laputa/snapshot` | 已存在 Tauri command，需在 `desktop.ts` 新增封装。 |
| `getLaputaSection(name)` | Tauri command + manager GET `/api/laputa/section/:name` | 已存在，确认 `desktop.ts` 已封装。 |
| `writeLaputaSection(name, content, summary?)` | **新增** Tauri command + manager POST `/api/laputa/section/:name/write` | 合成 proposal 并 apply，返回 ChangelogRecord。 |
| `listLaputaChangelog(filters)` | Tauri command + manager GET `/api/laputa/changelog` | 已存在，确认支持 `target_section` 过滤。 |

- `agent-diva-gui/src/api/desktop.ts` 必须暴露上述 4 个方法。
- manager 端新增 `POST /api/laputa/section/:name/write` handler；若与现有设计冲突，可改为 `POST /api/laputa/write` 并在 body 中携带 `target_section`。
- Tauri command 命名建议：`write_laputa_section`。

**验收**: GUI 能成功调用所有 4 个接口；未初始化的 `.laputa` 在首次保存时被创建。

### FR-107 — 空状态与错误处理
- `.laputa` 未初始化：显示“Laputa 尚未初始化，保存任意 section 后将自动创建”。
- section 读取失败：显示错误提示，提供“重试”按钮。
- 保存失败：在保存按钮旁或页面顶部显示错误信息，不清空用户输入。
- 网络/Tauri 桥接异常：降级为可点击重试。

**验收**: 手动构造读取失败，页面显示错误提示且用户输入保留。

### FR-108 — 保存后刷新
保存成功后自动刷新：
- 当前 section 内容；
- 左侧列表中的最后更新时间；
- 当前 section 的 changelog 计数（若有历史按钮 badge）。

同时提供手动刷新按钮。

**验收**: 保存后 2 秒内右侧内容与左侧时间同步更新。

## 6. Non-Functional Requirements

### NFR-101 — 性能
- 首次加载 `getLaputaSnapshot` 应在 500ms 内返回（本地文件场景）；
- section 切换加载应在 200ms 内；
- 历史弹窗加载 50 条记录应在 300ms 内。

### NFR-102 — 国际化
- 所有新增用户可见文案必须进入 `locales/zh.ts` 和 `locales/en.ts`；
- section 名称建议提供中英文映射表，不依赖后端返回的英文标识。

### NFR-103 — 可访问性
- 编辑器区域支持键盘聚焦；
- 保存按钮有明确禁用/启用状态；
- 历史弹窗焦点锁定在弹窗内，关闭后返回触发按钮。

### NFR-104 — 可维护性
- 新组件路径：`agent-diva-gui/src/components/PersonaMemoryView.vue`；
- 子组件建议拆分为：`SectionGroup.vue`、`SectionEditor.vue`、`HistoryModal.vue`；
- 新增 API 方法统一放在 `agent-diva-gui/src/api/desktop.ts`。

## 7. Open Questions / Deferred Items

| 项 | 状态 | 说明 |
|----|------|------|
| 回滚操作 | Deferred | 本期仅可复制历史内容，不做 rollback 按钮。已记入 TODOLIST.md。 |
| 保存前 diff | Deferred | 本期保存不展示 diff。已记入 TODOLIST.md。 |
| section 写权限矩阵 | Deferred | 当前全部可编辑；`agent_self` / `user_only` / `shared` 权限 UI 提示后续补齐。已记入 TODOLIST.md。 |
| 冲突检测 UI | Deferred | 后端已支持，本期遇到冲突直接报错误，后续再做可视化合并。 |
| 富文本/Markdown 预览 | Deferred | 本期仅源码编辑，后续可增加双栏预览。 |

## 8. Acceptance Criteria

- [ ] 在 GUI“功能”板块最上方看到“人格与记忆”菜单并点击进入；
- [ ] 左侧列出 14 个 section，按人格/记忆/周期/索引分组，TBD section 显示“待定”；
- [ ] 点击 `identity` 后右侧显示其 Markdown 内容；
- [ ] 修改内容后点击保存，2 秒内提示成功，左侧最后更新时间更新；
- [ ] 打开“历史”弹窗，能看到刚保存产生的 changelog 记录；
- [ ] 点击“复制该版本内容”后，剪贴板内容与记录中的 `after` 一致；
- [ ] `desktop.ts` 新增 `getLaputaSnapshot` 与 `writeLaputaSection` 封装；
- [ ] 中英文 locale 文件包含新增键值；
- [ ] 未初始化 `.laputa` 时首次保存能自动创建存储骨架。

## 9. Related Artifacts

- 底层 PRD: `docs/prds/prd-laputa-2026-06-12/prd.md`
- 消费方 PRD: `docs/prds/prd-selfinprove-2026-06-12/prd.md`
- 架构文档: `docs/architecture/evo-diva-architecture-2026-06-12.md`
- 实现参考页面:
  - `agent-diva-gui/src/components/EvolutionView.vue`（proposal 与 audit tab 布局参考）
  - `agent-diva-gui/src/components/NotebookView.vue`（双栏 list/detail 参考）
  - `agent-diva-gui/src/components/settings/MemoryChangelog.vue`（changelog 只读展示参考）
