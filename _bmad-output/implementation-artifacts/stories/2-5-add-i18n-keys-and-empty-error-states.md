# Story 2.5: Add i18n keys and empty/error states

Status: ready-for-dev

## Story

As a user,
I want clear messages when data is loading, missing, or failed,
So that I understand what is happening and what to do next.

## Acceptance Criteria

1. **Given** the page is loading,
   **When** skeletons are shown,
   **Then** they use `.skeleton-line` with the `skeleton-pulse` animation defined in `agent-diva-gui/src/styles.css`.

2. **Given** `.laputa/` is uninitialized,
   **When** the right panel renders,
   **Then** an empty state explains that saving any section will create the Laputa storage skeleton.

3. **Given** a section read failure,
   **When** the error is surfaced,
   **Then** an error state with an `AlertCircle` icon and a retry button is shown, and the user's draft (if any) is preserved.

4. **Given** a save failure,
   **When** the error is surfaced,
   **Then** an inline or banner error message is shown and the editor content remains editable.

5. **Given** the locale files are loaded,
   **When** the user switches between Chinese and English,
   **Then** all new strings render correctly in the chosen language under the `laputa.*` namespace.

6. **Given** the navigation sidebar is rendered,
   **When** the current locale changes,
   **Then** the "人格与记忆" / "Persona & Memory" label updates via `nav.personaMemory`.

## Tasks / Subtasks

- [ ] **Add `laputa.*` namespace to `agent-diva-gui/src/locales/zh.ts`** (AC: #5)
  - [ ] Open `agent-diva-gui/src/locales/zh.ts`
  - [ ] Insert the following object after the `notebook` block (or next to other page-level namespaces):
        ```ts
        laputa: {
          title: '人格与记忆',
          subtitle: '管理 Diva 的长期人格与记忆内容',
          loading: '正在加载 Laputa 数据…',
          loadError: '无法加载记忆数据',
          retry: '重试',
          save: '保存',
          saved: '已保存',
          saveFailed: '保存失败：{message}',
          saving: '保存中…',
          history: '历史',
          copy: '复制内容',
          copied: '已复制',
          emptyTitle: '此 section 还没有内容',
          emptyDesc: '在右侧编辑器中输入 Markdown 内容，然后点击保存。',
          uninitializedTitle: 'Laputa 尚未初始化',
          uninitializedDesc: '保存任意 section 后，系统将自动创建记忆骨架。',
          groups: {
            persona: '人格',
            memory: '记忆',
            periodic: '周期',
            indexes: '索引',
          },
          sections: {
            identity: '身份',
            relationship: '关系',
            commitment: '承诺',
            preferences: '偏好',
            memory_md: '记忆文档',
            history_md: '历史文档',
            daily: '日报',
            weekly: '周报',
            monthly: '月报',
            journal_reflective: '反思日志',
            proposal_inbox: '提案收件箱',
            changelog: '变更日志',
            report_indexes: '报告索引',
            aaak_summaries: 'AAAK 摘要',
          },
          status: {
            owned: '已就绪',
            tbd: '待定',
          },
          confirmSave: {
            title: '确认保存',
            message: '确定要覆盖 {section} 的当前内容吗？此操作会生成一条审计记录。',
            confirm: '确认覆盖',
            cancel: '取消',
          },
          confirmDiscard: {
            title: '确认放弃修改',
            message: '当前 section 有未保存的修改，切换后将丢失。是否放弃？',
            discard: '放弃',
            cancel: '取消',
          },
          historyModal: {
            title: '{section} 的变更历史',
            empty: '暂无变更记录',
          },
        },
        ```

- [ ] **Add `laputa.*` namespace to `agent-diva-gui/src/locales/en.ts`** (AC: #5)
  - [ ] Open `agent-diva-gui/src/locales/en.ts`
  - [ ] Insert the English equivalent after the `notebook` block:
        ```ts
        laputa: {
          title: 'Persona & Memory',
          subtitle: 'Manage Diva\'s long-term persona and memory content',
          loading: 'Loading Laputa data…',
          loadError: 'Could not load memory data',
          retry: 'Retry',
          save: 'Save',
          saved: 'Saved',
          saveFailed: 'Save failed: {message}',
          saving: 'Saving…',
          history: 'History',
          copy: 'Copy content',
          copied: 'Copied',
          emptyTitle: 'This section has no content yet',
          emptyDesc: 'Enter Markdown content in the editor, then click Save.',
          uninitializedTitle: 'Laputa is not initialized',
          uninitializedDesc: 'Save any section and the memory skeleton will be created automatically.',
          groups: {
            persona: 'Persona',
            memory: 'Memory',
            periodic: 'Periodic',
            indexes: 'Indexes',
          },
          sections: {
            identity: 'Identity',
            relationship: 'Relationship',
            commitment: 'Commitment',
            preferences: 'Preferences',
            memory_md: 'Memory Doc',
            history_md: 'History Doc',
            daily: 'Daily',
            weekly: 'Weekly',
            monthly: 'Monthly',
            journal_reflective: 'Reflective Journal',
            proposal_inbox: 'Proposal Inbox',
            changelog: 'Changelog',
            report_indexes: 'Report Indexes',
            aaak_summaries: 'AAAK Summaries',
          },
          status: {
            owned: 'owned',
            tbd: 'tbd',
          },
          confirmSave: {
            title: 'Confirm Save',
            message: 'Overwrite current content for {section}? This will create an audit record.',
            confirm: 'Overwrite',
            cancel: 'Cancel',
          },
          confirmDiscard: {
            title: 'Discard unsaved changes?',
            message: 'This section has unsaved changes. Switching will lose them. Discard?',
            discard: 'Discard',
            cancel: 'Cancel',
          },
          historyModal: {
            title: 'Change history for {section}',
            empty: 'No change records yet',
          },
        },
        ```

- [ ] **Add `nav.personaMemory` to both locale files** (AC: #6)
  - [ ] In `zh.ts`, add `personaMemory: '人格与记忆'` inside `nav:`.
  - [ ] In `en.ts`, add `personaMemory: 'Persona & Memory'` inside `nav:`.

- [ ] **Create `PersonaMemoryEmptyState.vue`** (AC: #2)
  - [ ] Create `agent-diva-gui/src/components/persona-memory/PersonaMemoryEmptyState.vue`
  - [ ] Accept props: `icon` (Lucide component), `title` (string), `description` (string).
  - [ ] Render a centered flex column with:
        - Icon sized `48px` and colored `var(--text-muted)` at `opacity: 0.5`.
        - Title using `text-base font-semibold` and `var(--text-muted)`.
        - Description using `text-sm` and `var(--text-muted)`.
  - [ ] Use this component for both the uninitialized state and the section-empty state by varying the props.

- [ ] **Create `PersonaMemoryErrorState.vue`** (AC: #3)
  - [ ] Create `agent-diva-gui/src/components/persona-memory/PersonaMemoryErrorState.vue`
  - [ ] Accept props: `title` (string), `message` (string, optional), `onRetry` (function).
  - [ ] Render a centered flex column with:
        - `AlertCircle` icon from `lucide-vue-next` sized `48px` and colored `var(--danger)`.
        - Title in `var(--text)`.
        - Optional message in `var(--text-muted)`.
        - Secondary-styled retry button with label `t('laputa.retry')`.

- [ ] **Add skeleton loading pattern to `PersonaMemoryView.vue`** (AC: #1)
  - [ ] In `agent-diva-gui/src/components/PersonaMemoryView.vue`:
    - While `loadingSnapshot === true`, render placeholder rows in the left panel using `<div class="skeleton-line" />`.
    - While `loadingSection === true`, render placeholder lines in the right panel using `<div class="skeleton-line" />`.
  - [ ] Ensure the `skeleton-line` class already exists in `agent-diva-gui/src/styles.css` (see DESIGN.md); if not, add it:
        ```css
        .skeleton-line {
          height: 12px;
          border-radius: 4px;
          background: var(--accent-bg-light);
          animation: skeleton-pulse 1.5s ease-in-out infinite;
        }
        @keyframes skeleton-pulse {
          0%, 100% { opacity: 0.4; }
          50% { opacity: 0.8; }
        }
        ```

- [ ] **Wire empty/error/skeleton states into `PersonaMemoryView.vue`** (AC: #1, #2, #3, #4)
  - [ ] Track state: `loadingSnapshot`, `loadingSection`, `sectionError`, `saveError`.
  - [ ] On initial load: show skeletons and a centered spinner (re-use `app.loading` or `laputa.loading`).
  - [ ] On snapshot load error: render `PersonaMemoryErrorState` with `title = t('laputa.loadError')` and retry action `loadSnapshot()`.
  - [ ] On uninitialized `.laputa/`: render `PersonaMemoryEmptyState` with `Inbox` or `BookOpen` icon, `title = t('laputa.uninitializedTitle')`, `description = t('laputa.uninitializedDesc')`.
  - [ ] On section read error: render `PersonaMemoryErrorState` with retry action `loadSection(selectedSection)`.
  - [ ] On save error: show inline banner or toast using `t('laputa.saveFailed', { message: errorMessage })`; do not clear `draftContent`.

- [ ] **Add save/error feedback to `SectionEditor.vue`** (AC: #4)
  - [ ] Display `saving` state on the Save button (label switches to `t('laputa.saving')`).
  - [ ] On failure, surface `saveError` below the toolbar or via the page-level error slot.

- [ ] **Run validation gates**
  - [ ] `cd agent-diva-gui && pnpm exec vue-tsc --noEmit`
  - [ ] `cd agent-diva-gui && pnpm exec eslint src/locales src/components/PersonaMemoryView.vue src/components/persona-memory`
  - [ ] `just fmt-check`
  - [ ] `just check`
  - [ ] Manual smoke test: launch GUI, switch language, open "人格与记忆", verify labels and empty states.

## Dev Notes

### Relevant architecture patterns and constraints

- **Locale-first strings**. All new user-visible text must live in `locales/zh.ts` and `locales/en.ts`. Do not hard-code Chinese or English strings in Vue templates. Section display names must come from `t('laputa.sections.<name>')`, not from raw backend identifiers.
- **Skeleton reuse**. The `skeleton-line` / `skeleton-pulse` animation is already specified in the UX contract (`DESIGN.md`). Verify it exists in `styles.css`; if missing, add it there so other pages can reuse it.
- **Empty/error components are pure presentational**. They receive props and emit retry events; they do not call `desktop.ts` directly. `PersonaMemoryView.vue` owns the load/retry logic.
- **Preserve user input on failure**. When a save or section load fails, do not reset `draftContent` or `selectedSection`.
- **Uninitialized state is not an error**. Use the friendly empty-state component with an informational icon, not the error-state component, when `.laputa/` has not been created yet.

### Source tree components to touch

- `agent-diva-gui/src/locales/zh.ts` — add `nav.personaMemory` and `laputa.*` namespace
- `agent-diva-gui/src/locales/en.ts` — add `nav.personaMemory` and `laputa.*` namespace
- `agent-diva-gui/src/styles.css` — confirm/add `.skeleton-line` and `@keyframes skeleton-pulse`
- `agent-diva-gui/src/components/persona-memory/PersonaMemoryEmptyState.vue` — new empty-state presentational component
- `agent-diva-gui/src/components/persona-memory/PersonaMemoryErrorState.vue` — new error-state presentational component
- `agent-diva-gui/src/components/PersonaMemoryView.vue` — wire loading / empty / error states
- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` — saving label and save-error display
- `agent-diva-gui/src/components/NormalMode.vue` — sidebar label should use `t('nav.personaMemory')`

### Testing standards summary

- **Locale coverage**: ensure every key added to `zh.ts` has a matching key in `en.ts` (and vice versa). A missing key will fall back to the key path, which is a UI regression.
- **TypeScript**: run `vue-tsc --noEmit` inside `agent-diva-gui` to catch missing locale keys referenced in templates.
- **Manual QA**:
  1. Start the GUI with a workspace where `.laputa/` does not exist.
  2. Open "人格与记忆" and confirm the uninitialized empty state appears.
  3. Switch language to English and confirm labels update.
  4. Force a section read failure (e.g., stop gateway or corrupt request) and confirm the error state with retry appears.
  5. Trigger a save failure and confirm the error message appears and the editor content is preserved.

### Project Structure Notes

- This story belongs to **Epic 2** (page skeleton and browsing). It can be developed in parallel with Story 2.3 (grouped list) and Story 2.4 (view content), but the locale keys added here must be available before those stories are finalized.
- The empty/error components are intentionally small and reusable; future Epic 3 stories (history modal, save flow) will rely on the `laputa.*` keys added here.

### References

- Empty/error state UX contract: [Source: docs/ux/persona-memory-laputa-2026-07-05/DESIGN.md §Empty State / §Skeleton]
- Experience copy guidance: [Source: docs/ux/persona-memory-laputa-2026-07-05/EXPERIENCE.md §Voice and Tone / §Component Patterns]
- PRD FR-107 / FR-108 / NFR-102: [Source: docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md]
- Architecture constraints on frontend ownership of grouping and strings: [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md AD-3]
- Existing locale structure and navigation namespace: [Source: agent-diva-gui/src/locales/zh.ts, agent-diva-gui/src/locales/en.ts]
- Reference pages for list-detail empty-state patterns: [Source: agent-diva-gui/src/components/NotebookView.vue, agent-diva-gui/src/components/EvolutionView.vue]

## Dev Agent Record

### Agent Model Used

(To be filled during implementation)

### Debug Log References

(To be filled during implementation)

### Completion Notes List

- [ ] `laputa.*` namespace added to both locale files with all requested keys
- [ ] `nav.personaMemory` added to both locale files and used in `NormalMode.vue`
- [ ] `PersonaMemoryEmptyState.vue` created and used for uninitialized / empty section states
- [ ] `PersonaMemoryErrorState.vue` created with `AlertCircle` icon and retry action
- [ ] `.skeleton-line` / `skeleton-pulse` pattern applied during loading
- [ ] Save failure preserves draft content and surfaces `laputa.saveFailed`
- [ ] `vue-tsc --noEmit` and lint pass inside `agent-diva-gui`
- [ ] Manual smoke test passed for locale switching and empty/error states

### File List

- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src/locales/en.ts`
- `agent-diva-gui/src/styles.css` (if skeleton CSS needs to be added)
- `agent-diva-gui/src/components/persona-memory/PersonaMemoryEmptyState.vue`
- `agent-diva-gui/src/components/persona-memory/PersonaMemoryErrorState.vue`
- `agent-diva-gui/src/components/PersonaMemoryView.vue`
- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue`
- `agent-diva-gui/src/components/NormalMode.vue`
