# Story 3.4: Implement changelog history modal

Status: ready-for-dev

## Story

As a user,
I want to view the change history of a Laputa section and copy any previous version,
So that I can recover earlier content without leaving the Persona & Memory page.

## Acceptance Criteria

1. **Given** a section is selected in `SectionEditor.vue`,
   **When** the user clicks the "历史" / "History" button,
   **Then** `HistoryModal.vue` opens and calls `listLaputaChangelog({ section: sectionName, limit: 50 })` from `agent-diva-gui/src/api/desktop.ts`.

2. **Given** the changelog request succeeds,
   **When** the modal renders the list,
   **Then** up to 50 records are displayed, each showing timestamp, action, actor, and an excerpt of the record's `after` content.

3. **Given** a changelog entry is visible,
   **When** the user clicks the "复制内容" / "Copy" button for that entry,
   **Then** the record's full `after` content is written to the clipboard and the button text temporarily changes to "已复制" / "Copied" for 1 second.

4. **Given** the modal is open,
   **When** the user clicks the scrim, clicks the X button, or presses Escape,
   **Then** the modal emits `close`, the overlay disappears, and focus returns to the History trigger button.

5. **Given** the modal is open,
   **When** the user presses Tab or Shift+Tab,
   **Then** focus remains trapped inside the modal (scrim itself is not focusable; focus cycles among focusable elements inside the modal card).

6. **Given** the modal is implemented,
   **When** GUI smoke tests are run,
   **Then** records render, copy behavior updates the button label and clipboard, and Escape/scrim close dismiss the modal.

## Tasks / Subtasks

- [ ] **Create `HistoryModal.vue` component** (AC: #1, #2, #4, #5)
  - [ ] Open/create `agent-diva-gui/src/components/persona-memory/HistoryModal.vue`
  - [ ] Define props and emits:
        ```vue
        <script setup lang="ts">
        defineProps<{
          sectionName: string;
          open: boolean;
        }>();
        defineEmits<{
          (e: 'close'): void;
        }>();
        </script>
        ```
  - [ ] Import `listLaputaChangelog` and `ChangelogRecord` from `agent-diva-gui/src/api/desktop.ts`.
  - [ ] Watch `open`: when it becomes `true`, fetch the changelog:
        ```ts
        import { listLaputaChangelog, type ChangelogRecord } from '@/api/desktop';

        const records = ref<ChangelogRecord[]>([]);
        const loading = ref(false);
        const error = ref<string | null>(null);

        watch(() => props.open, async (isOpen) => {
          if (!isOpen) return;
          loading.value = true;
          error.value = null;
          try {
            const page = await listLaputaChangelog({ section: props.sectionName, limit: 50 });
            records.value = page.items.slice(0, 50);
          } catch (err) {
            error.value = String(err);
          } finally {
            loading.value = false;
          }
        });
        ```
  - [ ] Render the scrim and modal card per UX-DR5:
        ```vue
        <template>
          <Teleport to="body">
            <Transition name="modal-fade">
              <div
                v-if="open"
                class="fixed inset-0 bg-black/45 backdrop-blur-[2px] z-[600] flex items-center justify-center p-6"
                @click.self="$emit('close')"
              >
                <div
                  class="w-full max-w-[640px] bg-[var(--panel-solid)] rounded-[var(--radius)] shadow-[0_20px_60px_rgba(0,0,0,0.22)] flex flex-col max-h-[calc(100vh-48px)]"
                  role="dialog"
                  aria-modal="true"
                  :aria-label="t('laputa.historyModal.title', { section: sectionDisplayName })"
                >
                  <!-- header, list, empty state -->
                </div>
              </div>
            </Transition>
          </Teleport>
        </template>
        ```
  - [ ] Render each record with timestamp, action badge, actor, and a 120-character excerpt of `after`:
        ```vue
        <div
          v-for="record in records"
          :key="record.id"
          class="border-b border-[var(--line)] last:border-b-0 py-3"
        >
          <div class="flex items-center justify-between gap-3 mb-1">
            <span class="text-xs text-[var(--text-muted)]">{{ formatDate(record.created_at) }}</span>
            <span class="badge">{{ record.action }}</span>
            <span class="text-xs text-[var(--text-muted)]">{{ record.applied_by }}</span>
          </div>
          <p class="text-sm text-[var(--text)] line-clamp-2">{{ excerpt(record.after) }}</p>
          <button class="secondary-button" @click="copyAfter(record)">
            {{ copiedId === record.id ? t('laputa.historyModal.copied') : t('laputa.historyModal.copy') }}
          </button>
        </div>
        ```
  - [ ] Implement copy behavior:
        ```ts
        const copiedId = ref<string | null>(null);
        async function copyAfter(record: ChangelogRecord) {
          try {
            await navigator.clipboard.writeText(record.after);
            copiedId.value = record.id;
            window.setTimeout(() => { copiedId.value = null; }, 1000);
          } catch (err) {
            // Optionally show toast; do not throw.
          }
        }
        ```
  - [ ] Add close button in the modal header that emits `close`.
  - [ ] Add `keydown.esc` handler on `document` (or the modal wrapper) that emits `close` while `open` is true.
  - [ ] Implement focus trap:
    - When `open` becomes true, wait for the next DOM tick, then focus the first focusable element inside the modal (e.g. the close button).
    - Track the first and last focusable elements inside the modal card.
    - On `Tab` from the last focusable element, move focus to the first; on `Shift+Tab` from the first, move focus to the last.
    - On `close`, the parent `SectionEditor.vue` restores focus to the History trigger button via its own template ref.

- [ ] **Add `laputa.historyModal.*` i18n keys** (AC: #2, #3)
  - [ ] Open `agent-diva-gui/src/locales/zh.ts`
  - [ ] Add under the existing `laputa` namespace:
        ```ts
        historyModal: {
          title: '{section} 的变更历史',
          empty: '暂无变更记录',
          copy: '复制内容',
          copied: '已复制',
          close: '关闭',
          loadError: '加载历史失败',
          retry: '重试',
        },
        ```
  - [ ] Open `agent-diva-gui/src/locales/en.ts`
  - [ ] Add the English equivalents:
        ```ts
        historyModal: {
          title: 'Change history for {section}',
          empty: 'No changelog records yet',
          copy: 'Copy content',
          copied: 'Copied',
          close: 'Close',
          loadError: 'Failed to load history',
          retry: 'Retry',
        },
        ```

- [ ] **Wire `HistoryModal` into `SectionEditor.vue`** (AC: #1, #4, #5)
  - [ ] Open `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` (delivered by Stories 2.4 / 3.1).
  - [ ] Import `HistoryModal` and conditionally render it:
        ```vue
        <script setup lang="ts">
        import { ref, nextTick } from 'vue';
        import HistoryModal from './HistoryModal.vue';

        const historyOpen = ref(false);
        const historyTriggerRef = ref<HTMLButtonElement | null>(null);

        function onHistoryClose() {
          historyOpen.value = false;
          nextTick(() => historyTriggerRef.value?.focus());
        }
        </script>
        ```
  - [ ] Add the History button in the toolbar and close handler to restore focus:
        ```vue
        <button
          ref="historyTriggerRef"
          class="secondary-button"
          type="button"
          @click="historyOpen = true"
        >
          {{ t('laputa.history') }}
        </button>
        <HistoryModal
          :open="historyOpen"
          :section-name="selectedSection"
          @close="onHistoryClose"
        />
        ```
  - [ ] Ensure the History button is disabled while the section is loading or uninitialized.

- [ ] **Run GUI smoke tests** (AC: #3, #6)
  - [ ] Start the GUI with `cd agent-diva-gui && pnpm tauri dev` (or run against a manager instance with a seeded `.laputa/` workspace).
  - [ ] Navigate to "人格与记忆" / "Persona & Memory".
  - [ ] Select `identity`, click "历史" / "History".
  - [ ] Verify the modal opens and:
    - records render with timestamp, action, actor, and excerpt;
    - clicking "复制内容" writes the full `after` content to the clipboard and the button label becomes "已复制" / "Copied" for 1 second;
    - pressing Escape closes the modal;
    - clicking the scrim closes the modal;
    - clicking the X button closes the modal;
    - focus returns to the History button after close.
  - [ ] Run `cd agent-diva-gui && pnpm vue-tsc --noEmit` to verify TypeScript.
  - [ ] Run `just fmt-check` and `just check` from the workspace root (Rust side is unchanged, but verify no regressions).

## Dev Notes

### Relevant architecture patterns and constraints

- **History modal is read-only with copy support only** (Architecture AD-5, PRD §4.3). Do not add rollback buttons, diff previews, or conflict resolution UI in this story. Rollback and diff preview are explicitly deferred.
- **All changelog data comes from `listLaputaChangelog`** (PRD FR-105, Architecture §4.1). The modal must not introduce a new persistence format or query path.
- **The `listLaputaChangelog` wrapper must support section filtering**. The current `desktop.ts` signature is positional (`page?, pageSize?, proposalId?`). If Story 1.4 has not yet extended it, update the wrapper in this story to accept an options object `{ section?: string; limit?: number; page?: number; proposalId?: string }` and map `section` to the backend `target_section` query parameter and `limit` to `page_size`.
- **Clipboard behavior must match `ChatView.vue`** (Architecture AD-5). Use `navigator.clipboard.writeText` and guard against rejection; temporarily swap the button label for 1 second.
- **Focus management is a hard requirement** (UX-DR15, Story 3.5 AC). The modal must trap focus and return focus to the History trigger button on close. Do not rely on the browser default for `<dialog>` unless the project already uses it consistently.
- **Modal styling must use CSS variables from `styles.css`** (UX-DR1). Do not hard-code colors, radii, or shadows.

### Source tree components to touch

- `agent-diva-gui/src/components/persona-memory/HistoryModal.vue` — new modal component (this story)
- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` — add History button and wire modal (delivered by Stories 2.4 / 3.1)
- `agent-diva-gui/src/api/desktop.ts` — ensure `listLaputaChangelog` accepts `{ section, limit }` filters ( Story 1.4 prerequisite; update here if needed)
- `agent-diva-gui/src/locales/zh.ts` — add `laputa.historyModal.*` keys
- `agent-diva-gui/src/locales/en.ts` — add `laputa.historyModal.*` keys
- No Rust/backend changes in this story

### Testing standards summary

- GUI smoke tests are required for user-visible changes (Project Rulebook `gui-changes-need-gui-smoke`).
- Verify focus trap with keyboard navigation (Tab, Shift+Tab, Escape).
- Verify clipboard behavior with real copy; if clipboard API is unavailable in the test environment, stub `navigator.clipboard.writeText` and assert the label change.
- Run `pnpm vue-tsc --noEmit` before claiming completion.

### Project Structure Notes

- This story is part of Epic 3 and depends on Epic 2 (page skeleton + `SectionEditor.vue`) and Epic 1 (`listLaputaChangelog` wrapper in `desktop.ts`).
- `HistoryModal.vue` should be a pure presentational component: props in, events out, no global state.
- Keep the modal markup inside a `<Teleport to="body">` so it renders above the app shell and is not clipped by parent `overflow` styles.

### References

- PRD §4 (Scope Boundaries), §5 FR-105, §6 NFR-102/NFR-103, §8 Acceptance Criteria: [Source: docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md]
- Architecture AD-2, AD-5, §4.1 API Contract, §8 Frontend File Checklist: [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md]
- UX modal specs (UX-DR5), copy behavior (UX-DR13), focus/close behavior (UX-DR15), i18n namespace (UX-DR16): [Source: docs/ux/persona-memory-laputa-2026-07-05/DESIGN.md]
- Experience flow, component patterns, state patterns, accessibility floor: [Source: docs/ux/persona-memory-laputa-2026-07-05/EXPERIENCE.md]
- Epic 3 breakdown and Story 3.4/3.5 AC: [Source: _bmad-output/planning-artifacts/epics.md]
- Existing Laputa API wrapper and types (`ChangelogRecord`, `ChangelogPage`, `listLaputaChangelog`): [Source: agent-diva-gui/src/api/desktop.ts]
- `SectionEditor.vue` integration point: [Source: agent-diva-gui/src/components/persona-memory/SectionEditor.vue]

## Dev Agent Record

### Agent Model Used

(To be filled during implementation)

### Debug Log References

(To be filled during implementation)

### Completion Notes List

- [ ] `HistoryModal.vue` created with props `sectionName` / `open`, emits `close`
- [ ] `listLaputaChangelog({ section, limit: 50 })` called on open with up to 50 records rendered
- [ ] Each entry shows timestamp, action, actor, and `after` excerpt
- [ ] Copy button writes full `after` content and shows "已复制" / "Copied" for 1 second
- [ ] Modal closes via scrim click, X button, and Escape; focus returns to trigger button
- [ ] Focus trap implemented while modal is open
- [ ] `laputa.historyModal.*` keys added to `zh.ts` and `en.ts`
- [ ] `HistoryModal` wired into `SectionEditor.vue`
- [ ] GUI smoke tests passed and `pnpm vue-tsc --noEmit` clean

### File List

- `agent-diva-gui/src/components/persona-memory/HistoryModal.vue`
- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue`
- `agent-diva-gui/src/api/desktop.ts`
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src/locales/en.ts`
