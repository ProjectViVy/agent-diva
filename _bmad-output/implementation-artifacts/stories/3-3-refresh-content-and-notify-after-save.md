# Story 3.3: Refresh content and notify after save

Status: review

## Story

As a user,
I want immediate feedback after saving,
So that I know the change was persisted.

## Acceptance Criteria

1. **Given** the user has clicked Save,
   **When** the save succeeds,
   **Then** the editor reloads the section content.

2. **Given** the save succeeds,
   **When** the section content is reloaded,
   **Then** the left list updates the section's last-updated time.

3. **Given** the save succeeds,
   **When** the editor finishes reloading,
   **Then** a success toast `已保存` / `Saved` is shown.

4. **Given** the save succeeds,
   **When** the success toast is shown,
   **Then** `isDirty` resets to `false`.

5. **Given** the save fails,
   **When** the error is caught,
   **Then** the error is displayed and the user's draft is preserved.

## Tasks / Subtasks

- [x] **Wire the save handler in `PersonaMemoryView.vue`** (AC: #1, #2, #3, #4, #5)
  - [x] Open `agent-diva-gui/src/components/PersonaMemoryView.vue`
  - [x] Ensure the component imports:
    ```ts
    import { ref, computed } from 'vue';
    import { useI18n } from 'vue-i18n';
    import {
      getLaputaSection,
      getLaputaSnapshot,
      writeLaputaSection,
      type LaputaSectionName,
      type LaputaSection,
    } from '@/api/desktop';
    import { showAppToast } from '@/utils/appToast';
    ```
  - [x] Maintain reactive state:
    ```ts
    const selectedSection = ref<LaputaSectionName>('identity');
    const originalContent = ref('');
    const draftContent = ref('');
    const isDirty = computed(() => draftContent.value !== originalContent.value);
    const saving = ref(false);
    const saveError = ref<string | null>(null);
    const snapshot = ref<unknown>(null);
    ```
  - [x] Implement an async `loadSection(name: LaputaSectionName)` helper:
    - Call `const section: LaputaSection = await getLaputaSection(name);`
    - Convert content to a string: `const text = typeof section.content === 'string' ? section.content : JSON.stringify(section.content ?? '', null, 2);`
    - Set `originalContent.value = text;`
    - Set `draftContent.value = text;`
    - Clear `saveError.value = null;`
  - [x] Implement an async `loadSnapshot()` helper:
    - Call `snapshot.value = await getLaputaSnapshot();`
  - [x] Implement an async `handleSave()` handler wired to the Save button from `SectionEditor.vue`:
    ```ts
    async function handleSave() {
      if (!isDirty.value || saving.value) return;
      saving.value = true;
      saveError.value = null;

      try {
        await writeLaputaSection(selectedSection.value, draftContent.value);
        await loadSection(selectedSection.value);
        await loadSnapshot();
        showAppToast(t('laputa.saved'), 'success');
      } catch (err) {
        const message = err instanceof Error ? err.message : String(err);
        saveError.value = message;
        showAppToast(t('laputa.saveFailed', { message }), 'error');
      } finally {
        saving.value = false;
      }
    }
    ```
  - [x] Ensure `isDirty` is reset to `false` implicitly by `loadSection` aligning `draftContent` with `originalContent`.

- [x] **Ensure the left list refreshes its last-updated time** (AC: #2)
  - [x] Pass the updated `snapshot` ref down to `SectionGroupList.vue`.
  - [x] Confirm that `SectionGroupList.vue` derives the last-updated time from `snapshot.sections[name].last_modified`.
  - [x] Verify that `await loadSnapshot()` after a successful save causes the left list to re-render the changed section's timestamp.

- [x] **Display inline save errors without discarding draft** (AC: #5)
  - [x] In `SectionEditor.vue` (or inside `PersonaMemoryView.vue`), render `saveError` as an inline error banner when it is non-null.
  - [x] Provide a retry action that calls `handleSave()` again.
  - [x] Do **not** mutate `draftContent` or `originalContent` in the error branch.

- [x] **Add required i18n keys** (AC: #3, #5)
  - [x] In `agent-diva-gui/src/locales/zh.ts`, add or confirm:
    ```ts
    laputa: {
      saved: '已保存',
      saveFailed: '保存失败：{message}',
      // ... other keys
    }
    ```
  - [x] In `agent-diva-gui/src/locales/en.ts`, add or confirm:
    ```ts
    laputa: {
      saved: 'Saved',
      saveFailed: 'Save failed: {message}',
      // ... other keys
    }
    ```

- [x] **Add frontend tests for the save-refresh-notify flow** (AC: #1, #2, #3, #4, #5)
  - [x] Create or update `agent-diva-gui/src/components/PersonaMemoryView.test.ts` (repo convention places tests next to components).
  - [x] Mock `agent-diva-gui/src/api/desktop.ts`:
    - `getLaputaSection` returns a resolved `LaputaSection`.
    - `getLaputaSnapshot` returns a resolved snapshot object.
    - `writeLaputaSection` resolves to a changelog result.
  - [x] Mock `agent-diva-gui/src/utils/appToast.ts`:
    - Spy on `showAppToast`.
  - [x] Test success path:
    - Mount `PersonaMemoryView.vue` with a selected section and a dirty draft.
    - Trigger the save action.
    - Assert `writeLaputaSection` is called with the selected section and draft content.
    - Assert `getLaputaSection` is called again (content refresh).
    - Assert `getLaputaSnapshot` is called again (list timestamp refresh).
    - Assert `showAppToast` is called with the saved message and `'success'` tone.
    - Assert `isDirty` is `false` after the refresh.
  - [x] Test failure path:
    - Make `writeLaputaSection` reject with `new Error('disk full')`.
    - Trigger save.
    - Assert `showAppToast` is called with `t('laputa.saveFailed', { message: 'disk full' })` and `'error'` tone.
    - Assert the draft value is preserved.
    - Assert `isDirty` remains `true`.

- [x] **Run validation gates**
  - [x] `pnpm --prefix agent-diva-gui test:unit` (or the project's unit-test command) passes.
  - [x] `just fmt-check` passes.
  - [x] `just check` passes.
  - [x] If a GUI smoke test is feasible, run `pnpm tauri dev`, edit a section, save, and confirm the toast and timestamp update.

## Dev Notes

### Relevant architecture patterns and constraints

- **Governance-first writes**: `writeLaputaSection` (added in Story 1.4) proxies to the manager's `POST /api/laputa/section/:name/write` endpoint, which uses `LaputaService::create_and_apply_direct_edit`. The GUI must never write `.laputa/` files directly.
- **Refresh sequence matters**: On success, always `await writeLaputaSection(...)` first, then `await loadSection(...)`, then `await loadSnapshot()`, then toast. This guarantees the left list timestamp and the editor content reflect the persisted state before the user sees the success message.
- **Preserve draft on failure**: The error branch must only set `saveError` and call `showAppToast(..., 'error')`. Do not overwrite `draftContent`, and do not reset `isDirty`.
- **Dirty semantics**: `isDirty` is derived from `draftContent !== originalContent`. A successful `loadSection` realigns both values, so `isDirty` naturally becomes `false`.
- **Toast contract**: Use the existing `showAppToast(message, tone)` utility. The default duration is 2400 ms; do not change it unless UX explicitly requires it.

### Source tree components to touch

- `agent-diva-gui/src/components/PersonaMemoryView.vue` — main save handler and refresh orchestration
- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` — emits `save`, renders `saving`/`saveError` states
- `agent-diva-gui/src/components/persona-memory/SectionGroupList.vue` — receives refreshed `snapshot`
- `agent-diva-gui/src/api/desktop.ts` — consumed, not modified (must already export `getLaputaSnapshot`, `getLaputaSection`, `writeLaputaSection` from Story 1.4)
- `agent-diva-gui/src/utils/appToast.ts` — consumed, not modified
- `agent-diva-gui/src/locales/zh.ts` — `laputa.saved`, `laputa.saveFailed`
- `agent-diva-gui/src/locales/en.ts` — `laputa.saved`, `laputa.saveFailed`
- `agent-diva-gui/src/components/__tests__/PersonaMemoryView.spec.ts` — new or updated unit tests

### Testing standards summary

- Mock at the `desktop.ts` boundary so tests do not invoke real Tauri commands or the manager.
- Use `@vue/test-utils` to mount the container and trigger the save flow.
- Cover both success and failure paths.
- Verify that the toast utility receives the correct arguments, including interpolation values for `saveFailed`.

### Project Structure Notes

- This story builds on Epic 1.4 (`desktop.ts` wrappers) and Epic 2 / Epic 3.1–3.2 (page container, editor, dirty tracking).
- No backend, Tauri command, or manager handler changes are required in this story.

### References

- `writeLaputaSection` / `getLaputaSnapshot` wrappers: [Source: agent-diva-gui/src/api/desktop.ts]
- Toast utility: [Source: agent-diva-gui/src/utils/appToast.ts]
- Save flow UX spec: [Source: docs/ux/persona-memory-laputa-2026-07-05/EXPERIENCE.md §State Patterns / Flow 1]
- PRD FR-104 / FR-107 / FR-108: [Source: docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md]
- Architecture state-flow diagram: [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md §5]
- Epic source: [Source: _bmad-output/planning-artifacts/epics.md]

## Dev Agent Record

### Agent Model Used

(To be filled during implementation)

### Debug Log References

(To be filled during implementation)

### Completion Notes List

- [x] `PersonaMemoryView.vue` save handler reloads section and snapshot in the correct order.
- [x] Success toast calls `showAppToast(t('laputa.saved'), 'success')`.
- [x] Failure branch preserves `draftContent` and calls `showAppToast(t('laputa.saveFailed', { message }), 'error')`.
- [x] Left list timestamp updates after save through snapshot refresh.
- [x] `isDirty` resets to `false` after a successful save.
- [x] Unit tests for success and failure paths pass.
- [x] `just fmt-check && just check` clean.

### File List

- `agent-diva-gui/src/components/PersonaMemoryView.vue`
- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue`
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src/locales/en.ts`
- `agent-diva-gui/src/components/PersonaMemoryView.test.ts`
