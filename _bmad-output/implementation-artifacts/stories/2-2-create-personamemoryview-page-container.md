# Story 2.2: Create PersonaMemoryView page container

Status: review

## Story

As a user,
I want a dedicated page container for "人格与记忆",
So that the list/detail layout and global page state are organized in one place.

## Acceptance Criteria

1. **Given** the user navigates to "人格与记忆",
   **When** the page loads,
   **Then** `PersonaMemoryView.vue` renders a header with title and refresh button.

2. **Given** the page container is mounted,
   **When** the layout renders,
   **Then** it shows a left panel (280px) for the section list and a right panel (`flex: 1`) for the editor/detail area.

3. **Given** the page is loaded,
   **When** Laputa data is fetched,
   **Then** the component manages `selectedSection`, `snapshot`, `loading`, `error`, and `isDirty` as top-level Composition API refs.

4. **Given** the snapshot has loaded successfully,
   **When** no section is explicitly selected,
   **Then** `selectedSection` defaults to `'identity'`.

5. **Given** the user clicks a section in the left list,
   **When** `onSectionSelect(name)` is called,
   **Then** it updates `selectedSection` and triggers `loadSection(name)`.

6. **Given** the user clicks the refresh button,
   **When** `onRefresh()` is called,
   **Then** it reloads the snapshot and the currently selected section content.

7. **Given** a section load fails,
   **When** `loadSection(name)` rejects,
   **Then** `error` is set, `loading` is cleared, and the right panel displays an error state with a retry action.

8. **Given** the editor reports dirty state,
   **When** `isDirty` becomes `true`,
   **Then** the container propagates this to the right panel and guards section switching (via `appConfirm`).

9. **Given** the component is implemented,
   **When** `pnpm vue-tsc --noEmit` is run,
   **Then** `PersonaMemoryView.vue` type-checks cleanly and all imports resolve.

## Tasks / Subtasks

- [x] **Create `PersonaMemoryView.vue` shell** (AC: #1, #2, #3)
  - [x] Open `agent-diva-gui/src/components/PersonaMemoryView.vue` (new file).
  - [x] Use `<script setup lang="ts">` and Vue 3 Composition API.
  - [x] Import `ref`, `computed`, `onMounted`, `watch` from `vue` and `useI18n` from `vue-i18n`.
  - [x] Import layout icons from `lucide-vue-next` (e.g., `BookUser`, `RefreshCw`, `Loader2`, `AlertCircle`).
  - [x] Import placeholder sub-components:
    ```ts
    import SectionGroupList from './persona-memory/SectionGroupList.vue';
    import SectionEditor from './persona-memory/SectionEditor.vue';
    ```
    (These files may not exist yet; create them as minimal placeholders so the container compiles.)
  - [x] Import API helpers from `desktop.ts`:
    ```ts
    import { getLaputaSnapshot, getLaputaSection } from '../api/desktop';
    import type { LaputaSection, LaputaSectionName } from '../api/desktop';
    ```
  - [x] Import shared UI utilities:
    ```ts
    import { showAppToast } from '../utils/appToast';
    import { appConfirm } from '../utils/appDialog';
    ```

- [x] **Define page state and types** (AC: #3, #4)
  - [x] Add a local `LaputaSnapshot` interface:
    ```ts
    interface LaputaSnapshot {
      sections: Record<string, {
        status: 'owned' | 'tbd';
        last_modified?: string | null;
      }>;
    }
    ```
  - [x] Declare state refs:
    ```ts
    const selectedSection = ref<LaputaSectionName>('identity');
    const snapshot = ref<LaputaSnapshot | null>(null);
    const loading = ref(false);
    const error = ref('');
    const isDirty = ref(false);
    ```
  - [x] Add a `sectionContent` ref to hold the loaded `LaputaSection`:
    ```ts
    const sectionContent = ref<LaputaSection | null>(null);
    ```

- [x] **Implement data-loading methods** (AC: #1, #4, #5, #6, #7)
  - [x] Implement `async function loadSnapshot()`:
    - Set `loading.value = true` and `error.value = ''`.
    - Call `getLaputaSnapshot()` (Tauri) or return a minimal mock in browser preview.
    - Store result in `snapshot.value`.
    - On error, set `error.value` with a normalized message.
    - Always clear `loading.value`.
  - [x] Implement `async function loadSection(name: LaputaSectionName)`:
    - Set `loading.value = true` and `error.value = ''`.
    - Call `getLaputaSection(name)`.
    - Store result in `sectionContent.value`.
    - Reset `isDirty.value = false` after a successful load.
    - On error, set `error.value` and keep the previous `sectionContent` if desired.
    - Always clear `loading.value`.
  - [x] Implement `async function onSectionSelect(name: LaputaSectionName)`:
    - If `isDirty.value === true`, call `appConfirm(t('laputa.confirmDiscard.message'), { title: t('laputa.confirmDiscard.title'), confirmLabel: t('laputa.confirmDiscard.discard'), cancelLabel: t('laputa.confirmDiscard.cancel') })`.
    - If the user cancels, return early without changing selection.
    - Otherwise set `selectedSection.value = name` and await `loadSection(name)`.
  - [x] Implement `async function onRefresh()`:
    - Await `loadSnapshot()`.
    - If `selectedSection.value` is still valid in the new snapshot, await `loadSection(selectedSection.value)`.
    - Otherwise reset `selectedSection.value = 'identity'` and load it.

- [x] **Wire lifecycle and selection side effects** (AC: #4, #6)
  - [x] In `onMounted`, call `loadSnapshot()` and then `loadSection('identity')`.
  - [x] Watch `selectedSection` and trigger `loadSection` when it changes programmatically (optional; the select handler already loads).

- [x] **Build the layout template** (AC: #1, #2)
  - [x] Wrap the view in `.persona-memory-view` with `display: flex; flex-direction: column; height: 100%;`.
  - [x] Add a header row `.persona-memory-header` containing:
    - Title icon + `{{ t('laputa.title') }}`.
    - Refresh button that calls `onRefresh()` and shows a spinner while `loading` is true.
  - [x] Add the body `.persona-memory-body` with two children:
    - `.persona-memory-list` (left, 280px, `border-right: 1px solid var(--line)`).
    - `.persona-memory-detail` (right, `flex: 1`).
  - [x] Render global error banner at the top of the detail area when `error` is set, with a retry button.
  - [x] Render loading skeletons using `.skeleton-line` and `skeleton-pulse` when `loading && !snapshot`.

- [x] **Slot placeholder sub-components** (AC: #2, #5, #8)
  - [x] In the left panel, render `<SectionGroupList />` with props:
    - `:snapshot="snapshot"`
    - `:selected-section="selectedSection"`
    - `@select="onSectionSelect"`
  - [x] In the right panel, render `<SectionEditor />` with props:
    - `:section-name="selectedSection"`
    - `:section="sectionContent"`
    - `:loading="loading"`
    - `:error="error"`
    - `@update:dirty="isDirty = $event"`
    - `@refresh="loadSection(selectedSection)"`
    (The actual editor implementation is Story 3.1; this story only wires the container-to-child contract.)

- [x] **Add scoped styles using CSS variables** (AC: #1, #2)
  - [x] Use variables from `agent-diva-gui/src/styles.css`:
    - `var(--panel)`, `var(--panel-solid)`, `var(--line)`, `var(--text)`, `var(--text-muted)`
    - `var(--accent)`, `var(--accent-bg-light)`, `var(--accent-border)`, `var(--accent-glow)`
    - `var(--radius)`, `var(--radius-sm)`, `var(--shadow)`
  - [x] Match header/body spacing to `NotebookView.vue` and `EvolutionView.vue` (header ~52–60px, body flex 1, list width 280px).
  - [x] Reuse `.skeleton-line` and `@keyframes skeleton-pulse` from `NotebookView.vue`.

- [x] **Add `laputa.*` i18n keys** (AC: #1, #5, #6, #7)
  - [x] In `agent-diva-gui/src/locales/zh.ts`, add inside the default export:
    ```ts
    laputa: {
      title: '人格与记忆',
      subtitle: '管理 Diva 的长期人格与记忆内容',
      refresh: '刷新',
      loading: '正在加载 Laputa 数据…',
      loadError: '无法加载记忆数据',
      retry: '重试',
      emptyTitle: '此 section 还没有内容',
      emptyDesc: '在右侧编辑器中输入 Markdown 内容，然后点击保存。',
      uninitializedTitle: 'Laputa 尚未初始化',
      uninitializedDesc: '保存任意 section 后，系统将自动创建记忆骨架。',
      confirmDiscard: {
        title: '确认放弃修改',
        message: '当前 section 有未保存的修改，切换后将丢失。是否放弃？',
        cancel: '取消',
        discard: '放弃',
      },
    }
    ```
  - [x] In `agent-diva-gui/src/locales/en.ts`, add the English equivalents.

- [x] **Register the component in `NormalMode.vue`** (AC: #1)
  - [x] Import `PersonaMemoryView` in `agent-diva-gui/src/components/NormalMode.vue`.
  - [x] Add `'persona-memory'` to the `SidebarSection` union and `activeMenu` ref union.
  - [x] Add a `v-else-if="activeMenu === 'persona-memory'"` branch in the content area that renders `<PersonaMemoryView />`.
  - [x] Note: the actual sidebar menu entry is Story 2.1; this story only ensures the component can be mounted when `activeMenu === 'persona-memory'`.

- [x] **Validation**
  - [x] `pnpm vue-tsc --noEmit` inside `agent-diva-gui` passes.
  - [x] `pnpm lint` (or equivalent) passes.
  - [x] The component renders without runtime errors when mounted via `NormalMode.vue` in browser preview (sub-components may be placeholders).

## Dev Notes

### Relevant architecture patterns and constraints

- **Container owns global page state**. `PersonaMemoryView.vue` is the single source of truth for `selectedSection`, `snapshot`, `loading`, `error`, and `isDirty`. Child components receive these values via props and emit events to change them.
- **Sub-components are placeholders in this story**. `SectionGroupList.vue` and `SectionEditor.vue` do not need full implementations here. Create minimal `.vue` files that accept the documented props and emit documented events so the container type-checks and renders.
- **Follow existing page-container conventions**. Study `NotebookView.vue` (list/detail skeleton, left 280px panel, `.notebook-header`/`.notebook-body`) and `EvolutionView.vue` (header title block + refresh button, error banner, scoped CSS variables) for layout, state naming, and styling patterns.
- **All UI strings under `laputa.*`**. Do not scatter new strings in other namespaces. This story introduces the core `laputa.*` namespace used by later stories.
- **Browser-preview safety**. The container should not crash when `__TAURI_INTERNALS__` is absent. Use the existing `isTauriRuntime()` helper from `desktop.ts` or guard invokes with a runtime check, returning empty/mock data in browser preview.
- **Dirty-state guard**. If the editor is dirty, switching sections must prompt via `appConfirm`. This prevents accidental data loss before Story 3.5 is implemented.
- **No direct file writes**. This is a frontend story; persistence goes through `desktop.ts` helpers. Do not add any `fs` or raw write logic in the GUI component.

### Source tree components to touch

- `agent-diva-gui/src/components/PersonaMemoryView.vue` — new page container
- `agent-diva-gui/src/components/persona-memory/SectionGroupList.vue` — placeholder (full implementation in Story 2.3)
- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` — placeholder (full implementation in Story 3.1)
- `agent-diva-gui/src/components/NormalMode.vue` — add `persona-memory` branch and import
- `agent-diva-gui/src/locales/zh.ts` — add `laputa.*` keys
- `agent-diva-gui/src/locales/en.ts` — add English `laputa.*` keys

### Testing standards summary

- **Unit / render test**: Mount `PersonaMemoryView.vue` in isolation with mocked `desktop.ts` functions. Assert the header title, refresh button, and left/right panels exist.
- **Integration test**: Mount via `NormalMode.vue` with `activeMenu` forced to `'persona-memory'`. Assert the component renders and `loadSnapshot`/`loadSection` are called on mount.
- **Browser preview smoke**: Open the GUI in browser preview, manually set `activeMenu = 'persona-memory'`, and confirm the placeholder layout appears without runtime errors.

### Project Structure Notes

- This story belongs to **Epic 2: 人格与记忆页面骨架与 Section 浏览**. It depends on the API wrappers added in Epic 1 (Story 1.4) — specifically `getLaputaSnapshot` and `getLaputaSection` in `desktop.ts`.
- It precedes Story 2.3 (grouped section list), Story 3.1 (Markdown editor), and Story 3.4 (history modal), which will flesh out the placeholder sub-components.
- Keep the container focused on layout and state orchestration; do not implement editor Markdown rendering, history modal, or save flow in this story.

### References

- Example BMAD story structure: [Source: `_bmad-output/implementation-artifacts/stories/1-1-add-direct-edit-and-apply-to-laputa-service.md`]
- PRD: [Source: `docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md`]
- Architecture spine: [Source: `docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md`]
- UX design contract: [Source: `docs/ux/persona-memory-laputa-2026-07-05/DESIGN.md`]
- UX experience spec: [Source: `docs/ux/persona-memory-laputa-2026-07-05/EXPERIENCE.md`]
- Epic breakdown: [Source: `_bmad-output/planning-artifacts/epics.md`]
- Reference layout (list/detail + skeleton): [Source: `agent-diva-gui/src/components/NotebookView.vue`]
- Reference layout (header + refresh + error banner): [Source: `agent-diva-gui/src/components/EvolutionView.vue`]
- Reference editor state pattern: [Source: `agent-diva-gui/src/components/console/ConfigEditor.vue`]
- Project CSS variables: [Source: `agent-diva-gui/src/styles.css`]
- API types and wrappers: [Source: `agent-diva-gui/src/api/desktop.ts`]
- Parent mount point: [Source: `agent-diva-gui/src/components/NormalMode.vue`]
- i18n structure: [Source: `agent-diva-gui/src/locales/zh.ts`, `agent-diva-gui/src/locales/en.ts`]

## Dev Agent Record

### Agent Model Used

(To be filled during implementation)

### Debug Log References

(To be filled during implementation)

### Completion Notes List

- [x] `PersonaMemoryView.vue` created with `<script setup>` layout, state refs, and methods
- [x] Placeholder `SectionGroupList.vue` and `SectionEditor.vue` created
- [x] `NormalMode.vue` already wired from Story 2.1 to render the component for `activeMenu === 'persona-memory'`; no changes required
- [x] `laputa.*` i18n keys added to both `zh.ts` and `en.ts`
- [x] `vue-tsc --noEmit` and `pnpm build` pass
- [x] `just fmt-check` and `just check` pass
- [x] Note: `pnpm lint` script does not exist in `agent-diva-gui/package.json`; validated with `pnpm build` instead

### File List

- `agent-diva-gui/src/components/PersonaMemoryView.vue`
- `agent-diva-gui/src/components/persona-memory/SectionGroupList.vue`
- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue`
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src/locales/en.ts`
