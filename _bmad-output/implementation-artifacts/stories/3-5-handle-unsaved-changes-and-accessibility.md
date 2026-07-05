# Story 3.5: Handle unsaved changes and accessibility

Status: review

## Story

As a user,
I want to be warned before losing unsaved edits,
So that accidental section switches do not discard my work.

## Acceptance Criteria

1. **Given** the editor has unsaved changes (`isDirty === true`),
   **When** the user selects a different section in `SectionGroupList.vue` (or any other path that would change `selectedSection`),
    **Then** an `appConfirm` dialog is shown with title and message loaded from i18n keys `laputa.confirmDiscard.title` and `laputa.confirmDiscard.message`,
    **And** the dialog presents two buttons: "取消" / "Cancel" and "放弃" / "Discard" loaded from `laputa.confirmDiscard.cancel` and `laputa.confirmDiscard.discard`.

2. **Given** the unsaved-changes confirm dialog is open,
   **When** the user chooses "取消" / "Cancel",
   **Then** `selectedSection` remains unchanged, the current draft content is preserved, and the dialog closes.

3. **Given** the unsaved-changes confirm dialog is open,
   **When** the user chooses "放弃" / "Discard",
   **Then** `selectedSection` switches to the new section, the new section content is loaded, and `draftContent` / `originalContent` / `isDirty` are reset.

4. **Given** `HistoryModal.vue` is open,
   **When** the user presses `Tab` or `Shift+Tab`,
   **Then** focus remains trapped inside the modal (cycles from last focusable element back to the first, and vice-versa).

5. **Given** `HistoryModal.vue` is open,
   **When** the user presses `Escape`, clicks the modal scrim, or clicks the close button,
   **Then** the modal closes and focus returns to the history trigger button.

6. **Given** the Persona & Memory page is rendered,
   **When** the user navigates with the keyboard using `Tab`,
   **Then** all interactive elements (section list items, group toggle headers, save button, history button, editor textarea) are reachable,
   **And** each focused element shows a visible focus ring using existing CSS variables (e.g. `--accent`).

7. **Given** the implementation is complete,
   **When** GUI smoke tests are run,
   **Then** simulating a dirty state and switching sections shows the confirm dialog with the expected labels,
   **And** `Tab` navigation order is logical and focus rings are visible,
   **And** opening/closing the history modal traps and restores focus correctly.

## Tasks / Subtasks

- [x] **Wire up the unsaved-changes guard in `PersonaMemoryView.vue`** (AC: #1, #2, #3)
  - [x] Open `agent-diva-gui/src/components/PersonaMemoryView.vue`
  - [x] Locate (or add) the `selectSection(nextId: string)` async handler that is called whenever the user tries to change sections
  - [x] At the start of the handler, check `isDirty.value === true` and `nextId !== selectedSection.value`
  - [x] If dirty, await `appConfirm` from `agent-diva-gui/src/utils/appDialog.ts`:
        ```ts
        const confirmed = await appConfirm(
          t('laputa.confirmDiscard.message'),
          {
            title: t('laputa.confirmDiscard.title'),
            confirmLabel: t('laputa.confirmDiscard.discard'),
            cancelLabel: t('laputa.confirmDiscard.cancel'),
          },
        );
        ```
  - [x] If `confirmed === false`, return early and do not change `selectedSection`
  - [x] If `confirmed === true`, proceed to switch:
        - Set `selectedSection.value = nextId`
        - Clear any existing `error.value`
        - Reset `draftContent.value = ''` and `originalContent.value = ''`
        - Reset `isDirty.value = false`
        - Call the existing `loadSection(nextId)` routine
  - [x] Ensure the guard runs for all section-selection entry points (clicking a list item, keyboard selection, etc.)

- [x] **Ensure `SectionGroupList.vue` only emits selection events** (AC: #1)
  - [x] Open `agent-diva-gui/src/components/persona-memory/SectionGroupList.vue`
  - [x] Verify that clicking a section item emits `select(section.id)` and does not mutate parent state directly
  - [x] Verify that group toggle headers emit `toggleGroup(groupKey)` and do not own expanded/collapsed state beyond local UI
  - [x] Add `tabindex="0"`, `role="button"`, and keyboard activation (`Enter` / `Space`) if the items are not already native `<button>` elements

- [x] **Add/confirm i18n keys for the confirm dialog** (AC: #1, #2, #3)
  - [x] Open `agent-diva-gui/src/locales/zh.ts`
  - [x] Under the `laputa` namespace add (or confirm) the `confirmDiscard` block:
        ```ts
        confirmDiscard: {
          title: '确认放弃修改',
          message: '当前 section 有未保存的修改，切换后将丢失。是否放弃？',
          cancel: '取消',
          discard: '放弃',
        },
        ```
   - [x] Open `agent-diva-gui/src/locales/en.ts`
   - [x] Under the `laputa` namespace add (or confirm) the `confirmDiscard` block:
        ```ts
        confirmDiscard: {
          title: 'Discard unsaved changes?',
          message: 'This section has unsaved changes. Switching will lose them. Discard?',
          cancel: 'Cancel',
          discard: 'Discard',
        },
        ```
   - [x] Confirm that no other locale key collides with `laputa.confirmDiscard.*`

- [x] **Implement focus management in `HistoryModal.vue`** (AC: #4, #5)
  - [x] Open `agent-diva-gui/src/components/persona-memory/HistoryModal.vue`
  - [x] On open, record the element that had focus (`document.activeElement`) before the modal opened
  - [x] Add a `ref` to the modal card container and query its focusable children (e.g. `[role="button"], button, a, input, textarea, select, [tabindex]:not([tabindex="-1"])`)
  - [x] Implement `onKeydown` on the modal container:
        - `Tab`: if focus is on the last focusable element, `event.preventDefault()` and move focus to the first focusable element
        - `Shift+Tab`: if focus is on the first focusable element, `event.preventDefault()` and move focus to the last focusable element
        - `Escape`: close the modal
  - [x] On modal close, restore focus to the previously-focused history trigger button using the recorded element
  - [x] Ensure the focus-trap listener is removed when the modal is unmounted

- [x] **Add keyboard reachability and focus rings to all interactive elements** (AC: #6)
  - [x] In `SectionGroupList.vue`:
        - Use native `<button>` elements for section items and group toggle headers where possible, or add `tabindex="0"` + `role="button"` + keyboard handlers
        - Apply a visible focus style such as `focus-visible:outline focus-visible:outline-2 focus-visible:outline-[var(--accent)]`
  - [x] In `PersonaMemoryView.vue` / editor toolbar:
        - Ensure Save and History buttons are native `<button>` elements
        - Ensure the editor `<textarea>` has a visible focus ring (already required by UX-DR4; confirm it uses `--accent` glow)
  - [x] Avoid relying solely on browser defaults; explicitly use CSS variables so all themes (love/dark/default/miku) render a visible ring

- [x] **Add semantic accessibility labels** (AC: #6)
  - [x] In `locales/zh.ts` and `locales/en.ts`, add an `a11y` sub-namespace under `laputa` for screen-reader-only labels, e.g.:
        ```ts
        a11y: {
          sectionList: 'Laputa section list',
          sectionItem: 'Section {name}',
          editor: 'Laputa section editor',
          historyButton: 'View change history for {section}',
          saveButton: 'Save changes to {section}',
        },
        ```
  - [x] Apply `aria-label` / `:aria-label` bindings in `SectionGroupList.vue`, `SectionEditor.vue`, and `HistoryModal.vue`
  - [x] Apply `role="dialog"`, `aria-modal="true"`, and `aria-labelledby` pointing to the modal title on the `HistoryModal.vue` card

- [x] **Run GUI smoke tests** (AC: #7)
  - [x] Start the GUI with `pnpm tauri dev` (or browser preview if Tauri runtime is unavailable)
  - [x] Navigate to "人格与记忆" / "Persona & Memory"
  - [x] Modify the `identity` section content without saving
  - [x] Click `relationship` in the left list
  - [x] Verify the confirm dialog appears with title `laputa.confirmDiscard.title` and message `laputa.confirmDiscard.message`
  - [x] Click "取消" / "Cancel" and verify the editor stays on `identity` with the draft intact
  - [x] Click `relationship` again, then click "放弃" / "Discard" and verify the editor loads `relationship` and the Save button is disabled
  - [x] Press `Tab` repeatedly through the page and verify every interactive element is reachable with a visible focus ring
  - [x] Open the history modal, press `Tab` and verify focus cycles inside, press `Escape` and verify focus returns to the history button
  - [x] Run `just fmt-check` and `just check` from the workspace root

## Dev Notes

### Relevant architecture patterns and constraints

- **All section selection must flow through a single handler in `PersonaMemoryView.vue`**. The guard must live in the parent container because it owns `isDirty`, `selectedSection`, and `draftContent`. Do not let `SectionGroupList.vue` mutate parent state directly or bypass the confirm dialog.
- **`appConfirm` resolves `true` when the user confirms (clicks the primary action)** and `false` when they cancel or dismiss. Map "Discard" to the primary/confirm action and "Cancel" to the cancel action so the semantics match the helper's contract.
- **Do not prompt when `isDirty === false`**. Switching sections should remain instantaneous in the clean state to meet NFR-101 (section switch within 200ms).
- **Reset dirty state only after the user discards**. If the user cancels, leave `draftContent`, `originalContent`, and `isDirty` exactly as they were.
- **History modal focus trap is an overlay concern, not a routing concern**. Coordinate with Story 3.4 (HistoryModal.vue base implementation) so the focus-trap code wraps the existing list/copy/close UI without changing the changelog data flow.
- **Use existing CSS variables for focus rings** so the page supports love/dark/default/miku themes without hard-coded colors. See `DESIGN.md` §Colors and `styles.css` for `--accent`, `--accent-glow`, `--panel-solid`, etc.
- **Do not implement rollback or diff preview**. These are explicitly deferred per PRD §4.2 / Architecture §7.

### Source tree components to touch

- `agent-diva-gui/src/components/PersonaMemoryView.vue` — add `selectSection` guard and orchestrate dirty-state reset
- `agent-diva-gui/src/components/persona-memory/SectionGroupList.vue` — ensure items only emit `select`, add keyboard reachability/focus rings
- `agent-diva-gui/src/components/persona-memory/HistoryModal.vue` — add focus trap, Escape close, and focus restoration
- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` — verify textarea and toolbar buttons are keyboard accessible and use CSS-variable focus rings
- `agent-diva-gui/src/locales/zh.ts` — add/confirm `laputa.confirmDiscard.*` and `laputa.a11y.*` keys
- `agent-diva-gui/src/locales/en.ts` — add/confirm `laputa.confirmDiscard.*` and `laputa.a11y.*` keys
- `agent-diva-gui/src/utils/appDialog.ts` — no changes expected; consume the existing `appConfirm` API

### Testing standards summary

- GUI changes require manual smoke tests in addition to workspace `just ci` because automated Rust tests do not exercise Vue focus behavior.
- For the confirm dialog, assert both branches: cancel preserves state, discard switches and resets dirty state.
- For accessibility, test with keyboard only (no mouse) and verify that focus rings are visible on every interactive element.
- For the modal, assert Tab cycles, Escape closes, and focus returns to the trigger button (not `document.body`).

### Project Structure Notes

- This story depends on Story 2.3 (grouped section list), Story 3.2 (dirty tracking and save flow), and Story 3.4 (history modal). It does not create new components but adds behavior and polish to those components.
- No backend/API changes are required for this story; it is purely a frontend concern.
- Keep behavior localized to the Persona & Memory page. Do not change global dialog or toast behavior.

### References

- `appConfirm` API and dialog state contract: [Source: agent-diva-gui/src/utils/appDialog.ts]
- Existing locale structure and i18n patterns: [Source: agent-diva-gui/src/locales/zh.ts, agent-diva-gui/src/locales/en.ts]
- PRD functional requirements §4 / §5 (FR-104, FR-105, FR-107) and NFR-103: [Source: docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md]
- Architecture AD-2 (proposal-driven persistence) and AD-5 (read-only history modal): [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md]
- Visual tokens and focus styles: [Source: docs/ux/persona-memory-laputa-2026-07-05/DESIGN.md]
- Unsaved-changes flow, keyboard/accessibility floor, and UX-DR11 / UX-DR15: [Source: docs/ux/persona-memory-laputa-2026-07-05/EXPERIENCE.md]
- Epic & Story 3.5 source: [Source: _bmad-output/planning-artifacts/epics.md]
- Prior story context: Story 2.3 (group list), Story 3.2 (dirty tracking), Story 3.4 (history modal)

## Dev Agent Record

### Agent Model Used

(To be filled during implementation)

### Debug Log References

(To be filled during implementation)

### Completion Notes List

- [x] Unsaved-changes guard implemented in `PersonaMemoryView.vue` for all section-selection paths
- [x] `SectionGroupList.vue` emits selection events and section items are keyboard-reachable
- [x] `laputa.confirmDiscard.*` keys added to both `locales/zh.ts` and `locales/en.ts`
- [x] `laputa.a11y.*` labels added to both locale files and bound in components
- [x] `HistoryModal.vue` traps focus, closes on Escape/scrim/close button, and restores focus to trigger
- [x] All interactive elements show visible focus rings using CSS variables
- [x] GUI smoke tests passed for dirty-state confirm dialog, Tab navigation, and modal focus management
- [x] `just fmt-check && just check` clean

### File List

- `agent-diva-gui/src/components/PersonaMemoryView.vue`
- `agent-diva-gui/src/components/persona-memory/SectionGroupList.vue`
- `agent-diva-gui/src/components/persona-memory/HistoryModal.vue`
- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` (verification/focus-ring adjustments only)
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src/locales/en.ts`
