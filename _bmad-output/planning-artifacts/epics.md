---
stepsCompleted:
  - step-01
  - step-02
  - step-03
inputDocuments:
  - docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md
  - docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md
  - docs/ux/persona-memory-laputa-2026-07-05/DESIGN.md
  - docs/ux/persona-memory-laputa-2026-07-05/EXPERIENCE.md
---

# agent-diva-pro - Epic Breakdown: Persona & Memory

## Overview

This document provides the epic and story breakdown for the "Persona & Memory" Laputa GUI management page, decomposing requirements from the PRD, Architecture spine, and UX design contract into implementable stories.

## Requirements Inventory

### Functional Requirements

FR1: Add "Persona & Memory" menu item at the top of the GUI sidebar "Capabilities" section, with Chinese label "人格与记忆" and English label "Persona & Memory", routing to `activeMenu = 'persona-memory'` and mounting `PersonaMemoryView.vue`.

FR2: Display a grouped list of all 14 Laputa sections on the left side, organized into Persona (identity, relationship, commitment, preferences), Memory (memory_md, history_md), Periodic (daily, weekly, monthly), and Indexes (journal_reflective, proposal_inbox, changelog, report_indexes, aaak_summaries); default to selecting `identity` on first entry.

FR3: View the currently selected section's name, last updated time, and Markdown content in the right panel; handle uninitialized `.laputa/` state with a friendly empty-state message.

FR4: Provide a Markdown editor (textarea-based) where users can edit section content; enable the Save button only when content has changed; on save call `writeLaputaSection(name, content)` which internally creates an `EvolutionProposal`, transitions it, applies it, and generates an auditable changelog record.

FR5: Provide a "History" button for each section that opens a modal overlay showing the section's changelog list; each entry displays time, action, actor, and excerpt; a copy button writes the record's `after` content to the clipboard.

FR6: Expose or add the necessary backend APIs and frontend wrappers: `getLaputaSnapshot()` in `desktop.ts`, confirm `getLaputaSection(name)`, add new `writeLaputaSection(name, content)` Tauri command and manager `POST /api/laputa/section/:name/write` endpoint, and reuse `listLaputaChangelog(filters)`.

FR7: Implement empty states and error handling for uninitialized Laputa, section read failures, save failures, and Tauri/network exceptions; preserve user input on save failure and offer retry actions.

FR8: After a successful save, automatically refresh the current section content, the last-updated time in the left list, and any changelog badge; provide a manual refresh button.

### NonFunctional Requirements

NFR1: Performance — first `getLaputaSnapshot` load must complete within 500ms, section switch within 200ms, and history modal loading 50 records within 300ms in local-file scenarios.

NFR2: Internationalization — all new user-facing strings must be added to both `locales/zh.ts` and `locales/en.ts`; section display names must come from locale mappings, not raw backend identifiers.

NFR3: Accessibility — editor area must be keyboard focusable; Save button must have clear enabled/disabled states; history modal must trap focus and return focus to the trigger button on close.

NFR4: Maintainability — new page component at `agent-diva-gui/src/components/PersonaMemoryView.vue`; sub-components under `agent-diva-gui/src/components/persona-memory/`; new API wrappers in `agent-diva-gui/src/api/desktop.ts`.

### Additional Requirements

- The new page is a pure consumer of existing `agent-diva-laputa` and `agent-diva-manager` APIs; it must not introduce new persistence formats or database tables.
- Add a new high-level backend endpoint `POST /api/laputa/section/:name/write` (Tauri command `laputa_write_section`) that internally performs create proposal → transition to Approved → apply.
- The 14-section grouping model is a frontend UI concept and must not leak into backend types or API schemas.
- For v0.1.0 all sections are editable; no UI permission matrix is implemented yet.
- The history modal is read-only with copy support; rollback and diff preview are explicitly deferred.
- A new Markdown editor component must be created because the codebase has no existing textarea + preview Markdown editor.
- All file-system writes must go through `LaputaService`; manager/Tauri/GUI must not directly `fs::write` under `.laputa/sections` to keep `authority_boundary_guard` tests passing.
- The page must not overlap with `EvolutionView.vue` responsibilities; proposal approval, batch actions, and policy configuration remain in `EvolutionView.vue`.
- Component seed: `PersonaMemoryView.vue`, `SectionGroupList.vue`, `SectionEditor.vue`, `HistoryModal.vue`.
- Backend implementation order: `agent-diva-laputa/src/service.rs` → `agent-diva-manager/src/handlers/laputa.rs` → `agent-diva-manager/src/server.rs` → `agent-diva-gui/src-tauri/src/commands.rs` → `agent-diva-gui/src-tauri/src/lib.rs`.

### UX Design Requirements

UX-DR1: Support love/dark/default/miku themes using existing CSS variables from `agent-diva-gui/src/styles.css`.

UX-DR2: Section list item active state: 3px left accent border, `var(--panel-solid)` background, `var(--accent)` text, and subtle glow shadow.

UX-DR3: Section status badges: `owned` uses accent background/border/text; `tbd` uses transparent background with `var(--line)` border and `var(--text-muted)` text.

UX-DR4: Create a new Markdown editor component with a textarea (min-height 320px, resizable vertically) and a rendered Markdown preview pane using `markdown-it` + `highlight.js`.

UX-DR5: History modal overlay: `fixed inset-0 bg-black/45 backdrop-blur-[2px] z-[600]` scrim; modal card `max-w-[640px]` with `var(--panel-solid)` background and large shadow.

UX-DR6: Primary Save button uses `var(--accent)` background with white text; secondary History/Retry buttons use transparent background with `var(--line)` border.

UX-DR7: Empty states use centered icon + title + description in `var(--text-muted)`.

UX-DR8: Loading skeletons use `.skeleton-line` with `skeleton-pulse` animation.

UX-DR9: Default selected section on first entry is `identity`.

UX-DR10: Section groups are collapsible/expandable and default to fully expanded.

UX-DR11: If the editor has unsaved changes (`isDirty === true`), switching sections triggers an `appConfirm` dialog asking whether to discard changes.

UX-DR12: After a successful save, show `showAppToast(t('laputa.saved'), 'success')`.

UX-DR13: In the history modal, the copy button temporarily changes to "已复制" / "Copied" for 1 second after copying.

UX-DR14: When `.laputa/` is uninitialized, display an empty state explaining that saving any section will create the storage skeleton.

UX-DR15: History modal closes on Escape, traps focus while open, and returns focus to the trigger button on close.

UX-DR16: Add a new `laputa.*` i18n namespace in `locales/zh.ts` and `locales/en.ts` for page title, group names, section names, status labels, buttons, confirmations, and history modal copy.

### FR Coverage Map

| FR | Epic | 说明 |
|---|---|---|
| FR1 | Epic 2 | 侧边栏导航入口 |
| FR2 | Epic 2 | 14 section 分组列表 |
| FR3 | Epic 2 | section 内容查看 |
| FR4 | Epic 1 + Epic 3 | 后端写入通道 + 前端编辑器保存 |
| FR5 | Epic 3 | 历史弹窗与复制 |
| FR6 | Epic 1 | 后端 API 与 desktop.ts 封装 |
| FR7 | Epic 2 + Epic 3 | 空态与错误处理（浏览+编辑历史） |
| FR8 | Epic 3 | 保存后刷新 |

## Epic List

### Epic 1: 后端直接编辑与审计写入通道
**用户价值**：让“人格与记忆”页面有可靠、可审计的写入能力。

**FRs covered:** FR6, FR4 (backend portion), FR7/FR8 (backend foundation)

**Implementation Notes:**
- 在 `agent-diva-laputa/src/service.rs` 新增 `create_and_apply_direct_edit`；
- 在 `agent-diva-manager/src/handlers/laputa.rs` 新增 `write_laputa_section_handler`；
- 在 `agent-diva-manager/src/server.rs` 注册 `POST /api/laputa/section/:name/write`；
- 在 `agent-diva-gui/src-tauri/src/commands.rs` 新增 `laputa_write_section`；
- 在 `agent-diva-gui/src-tauri/src/lib.rs` 的 `generate_handler!` 中注册；
- 在 `agent-diva-gui/src/api/desktop.ts` 暴露 `getLaputaSnapshot` 与 `writeLaputaSection`；
- 确保 `agent-diva-laputa/tests/authority_boundary_guard.rs` 保持通过。

**完成标志：** GUI 能调用一个接口直接写入 section 并拿到 changelog。

### Epic 2: 人格与记忆页面骨架与 Section 浏览
**用户价值**：用户能在 GUI 中打开“人格与记忆”，浏览所有 14 个 section，并查看任一 section 的当前内容。

**FRs covered:** FR1, FR2, FR3, FR7/FR8 (frontend browsing portion)

**Implementation Notes:**
- 修改 `NormalMode.vue` 添加侧边栏“人格与记忆”入口（位于功能板块最上方）；
- 新增 `PersonaMemoryView.vue` 容器；
- 新增 `SectionGroupList.vue`（4 分组、14 section、状态徽章、可折叠）；
- 新增 `SectionEditor.vue` 只读/基础版（显示内容、最后更新时间）；
- 添加中英文 locale key：`nav.personaMemory`、分组名、section 名、状态标签、空态文案；
- 处理 `.laputa` 未初始化、section 读取错误等空态与重试。

**完成标志：** 用户能进入页面，看到分组列表，点击 section 查看内容。

### Epic 3: Section 编辑、保存与变更历史
**用户价值**：用户能直接编辑 section 内容并保存，同时查看和复制历史版本。

**FRs covered:** FR4 (frontend portion), FR5, FR7/FR8 (save feedback & refresh), NFR1-4, UX-DR4/DR5/DR11-DR16

**Implementation Notes:**
- 完成 `SectionEditor.vue` 的 Markdown textarea + preview pane；
- 实现 dirty 检测、保存按钮启用/禁用、保存前二次确认；
- 保存成功后刷新内容与左侧最后更新时间，并通过 `showAppToast` 反馈；
- 新增 `HistoryModal.vue`：拉取 section changelog、列表展示、复制 `after` 内容；
- 切换 section 时若存在未保存内容，弹出 `appConfirm` 确认是否放弃；
- 历史弹窗焦点管理、Escape 关闭、关闭后焦点返回触发按钮。

**完成标志：** 用户能编辑、保存、查看历史、复制历史内容。

**自然依赖：**
- Epic 1 必须在 Epic 3 之前完成（保存需要后端接口）。
- Epic 2 可与 Epic 1 并行启动，但 Epic 3 依赖 Epic 2 的页面骨架。

## Epic 1: 后端直接编辑与审计写入通道

**Epic Goal:** 让“人格与记忆”页面有可靠、可审计的写入能力，前端可以通过单一 API 直接保存 section 内容并生成 changelog。

### Story 1.1: Add direct-edit-and-apply method to LaputaService

As a developer,
I want a high-level `create_and_apply_direct_edit` method in `LaputaService`,
So that direct user edits can reuse the existing proposal governance flow without leaking raw file writes outside the crate.

**Acceptance Criteria:**

**Given** a valid `LaputaSectionName` and patch content
**When** `create_and_apply_direct_edit(name, patch, actor)` is called
**Then** it creates an `EvolutionProposal`, transitions it to `Approved`, applies it, and returns the resulting `ChangelogRecord`
**And** the method writes section content, changelog, and audit records atomically through `LaputaService`
**And** it does not expose raw filesystem access to callers outside `agent-diva-laputa`

### Story 1.2: Add manager endpoint for writing a section

As a developer,
I want a `POST /api/laputa/section/:name/write` endpoint in the manager,
So that the GUI can request auditable section writes over HTTP.

**Acceptance Criteria:**

**Given** the manager is running with a configured `LaputaService`
**When** a client sends `POST /api/laputa/section/:name/write` with `{ content, actor?, summary? }`
**Then** the handler calls `LaputaService::create_and_apply_direct_edit` and returns `{ changelog_id, applied_at }`
**And** invalid section names or malformed payloads return `400`/`422` with a clear error message
**And** the route is registered in `agent-diva-manager/src/server.rs`

### Story 1.3: Add Tauri command for writing a section

As a GUI developer,
I want a Tauri command `laputa_write_section`,
So that the frontend can invoke the new manager endpoint through the Tauri bridge.

**Acceptance Criteria:**

**Given** the GUI is running in Tauri mode
**When** `invoke('laputa_write_section', { name, content })` is called
**Then** the command proxies the request to `POST /api/laputa/section/:name/write`
**And** the command is registered in `agent-diva-gui/src-tauri/src/lib.rs` via `generate_handler!`
**And** errors from the manager are returned to the frontend without loss of detail

### Story 1.4: Expose Laputa API wrappers in desktop.ts

As a frontend developer,
I want `getLaputaSnapshot` and `writeLaputaSection` available in `desktop.ts`,
So that the new page can call Laputa APIs consistently with the rest of the GUI.

**Acceptance Criteria:**

**Given** `agent-diva-gui/src/api/desktop.ts`
**When** the module is imported by `PersonaMemoryView.vue`
**Then** it exports `getLaputaSnapshot()` wrapping `laputa_get_snapshot`
**And** it exports `writeLaputaSection(name, content, summary?)` wrapping `laputa_write_section`
**And** existing methods such as `getLaputaSection` and `listLaputaChangelog` continue to work unchanged

## Epic 2: 人格与记忆页面骨架与 Section 浏览

**Epic Goal:** 用户能在 GUI 中打开“人格与记忆”，浏览所有 14 个 section，并查看任一 section 的当前内容。

### Story 2.1: Add sidebar navigation entry

As a user,
I want a “人格与记忆” menu item at the top of the Capabilities section,
So that I can navigate to the new page.

**Acceptance Criteria:**

**Given** the GUI sidebar is visible
**When** I look at the “功能” (Capabilities) group
**Then** “人格与记忆” / “Persona & Memory” appears as the first item
**And** clicking it sets `activeMenu = 'persona-memory'` and renders `PersonaMemoryView.vue`
**And** the active menu highlight follows the selected section

### Story 2.2: Create PersonaMemoryView page container

As a user,
I want a dedicated page container for “人格与记忆”,
So that the list/detail layout and global page state are organized in one place.

**Acceptance Criteria:**

**Given** the user navigates to “人格与记忆”
**When** the page loads
**Then** `PersonaMemoryView.vue` renders a header with title and refresh button
**And** it shows a left panel for the section list and a right panel for the editor
**And** it manages `selectedSection`, `snapshot`, `loading`, `error`, and `isDirty` state

### Story 2.3: Implement grouped section list

As a user,
I want the 14 Laputa sections grouped into Persona, Memory, Periodic, and Indexes,
So that I can quickly find the section I want to edit.

**Acceptance Criteria:**

**Given** the snapshot has been loaded
**When** the section list renders
**Then** it displays 4 collapsible groups with the correct 14 sections
**And** each section shows its localized name, English key, and status badge (`owned` / `tbd`)
**And** groups default to expanded
**And** clicking a section selects it and loads its content

### Story 2.4: View section content

As a user,
I want to see the current Markdown content of a selected section,
So that I know what is stored before editing.

**Acceptance Criteria:**

**Given** a section is selected
**When** its content loads successfully
**Then** the right panel shows the section title, status badge, last updated time, and Markdown content
**And** the initial view uses a read-only rendering or simple text display
**And** switching sections refreshes the right panel within 200ms in local-file scenarios

### Story 2.5: Add i18n keys and empty/error states

As a user,
I want clear messages when data is loading, missing, or failed,
So that I understand what is happening and what to do next.

**Acceptance Criteria:**

**Given** the page is loading
**When** skeletons are shown
**Then** they use `.skeleton-line` with `skeleton-pulse` animation
**And** given `.laputa/` is uninitialized, an empty state explains that saving any section will create the skeleton
**And** given a read failure, an error state with a retry button is shown
**And** all new strings exist in both `locales/zh.ts` and `locales/en.ts` under a `laputa.*` namespace

## Epic 3: Section 编辑、保存与变更历史

**Epic Goal:** 用户能直接编辑 section 内容并保存，同时查看和复制历史版本。

### Story 3.1: Implement Markdown editor with preview

As a user,
I want a Markdown textarea with a live preview pane,
So that I can edit section content and see how it renders.

**Acceptance Criteria:**

**Given** a section is selected
**When** the editor renders
**Then** it shows a textarea (min-height 320px, vertically resizable) on one side and a rendered Markdown preview on the other
**And** the preview uses `markdown-it` + `highlight.js`
**And** the editor uses CSS variables from `styles.css` so all themes are supported

### Story 3.2: Implement save flow and dirty tracking

As a user,
I want the Save button to be enabled only when I have made changes,
So that I do not accidentally submit unchanged content.

**Acceptance Criteria:**

**Given** the editor has loaded the current section content
**When** the user types in the textarea
**Then** `isDirty` becomes true and the Save button enables
**And** given the content is unchanged, the Save button remains disabled
**And** when the user clicks Save, a confirmation dialog appears if the section already has content
**And** saving calls `writeLaputaSection(name, draftContent)`

### Story 3.3: Refresh content and notify after save

As a user,
I want immediate feedback after saving,
So that I know the change was persisted.

**Acceptance Criteria:**

**Given** the user has clicked Save
**When** the save succeeds
**Then** the editor reloads the section content
**And** the left list updates the section's last-updated time
**And** a success toast “已保存” / “Saved” is shown
**And** `isDirty` resets to false
**And** given a save failure, the error is displayed and the user's draft is preserved

### Story 3.4: Implement changelog history modal

As a user,
I want to view the change history of a section and copy any previous version,
So that I can recover earlier content.

**Acceptance Criteria:**

**Given** a section is selected
**When** the user clicks the “历史” / “History” button
**Then** a modal overlay opens with the section's changelog (max 50 records)
**And** each entry shows timestamp, action, actor, and excerpt
**And** clicking “复制内容” / “Copy” writes the record's `after` content to the clipboard
**And** the button temporarily changes to “已复制” / “Copied” for 1 second
**And** the modal closes on clicking the scrim, the X button, or pressing Escape

### Story 3.5: Handle unsaved changes and accessibility

As a user,
I want to be warned before losing unsaved edits,
So that accidental section switches do not discard my work.

**Acceptance Criteria:**

**Given** the editor has unsaved changes
**When** the user selects a different section
**Then** an `appConfirm` dialog asks whether to discard changes
**And** choosing “取消” / “Cancel” keeps the current section and draft
**And** choosing “放弃” / “Discard” switches to the new section
**And** the history modal traps focus while open and returns focus to the trigger button on close
**And** all interactive elements in the page are reachable via keyboard Tab

