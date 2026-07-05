# Story 3.2: Implement save flow and dirty tracking

Status: ready-for-dev

## Story

As a user,
I want the Save button to be enabled only when I have made changes,
So that I do not accidentally submit unchanged content.

## Acceptance Criteria

1. **Given** the editor has loaded the current section content,
   **When** the user types in the textarea,
   **Then** `isDirty` becomes `true` and the Save button enables.

2. **Given** the content is unchanged,
   **When** the user looks at the Save button,
   **Then** it remains disabled.

3. **Given** the section already has non-empty content and the user has made changes,
   **When** the user clicks Save,
   **Then** an `appConfirm` confirmation dialog appears asking whether to overwrite the existing content.

4. **Given** the section is empty (first write) or the user confirms overwrite,
   **When** saving proceeds,
   **Then** the component calls `writeLaputaSection(name, draftContent)` from `desktop.ts`.

5. **Given** a save is in progress,
   **When** the user looks at the Save button,
   **Then** it shows a loading spinner and is disabled to prevent duplicate submissions.

6. **Given** a save succeeds,
   **When** the response returns,
   **Then** `originalContent` is updated to the saved value, `draftContent` stays in sync, `isDirty` resets to `false`, and the Save button disables again.

7. **Given** a save fails,
   **When** the error returns,
   **Then** the error is surfaced without discarding `draftContent`, the Save button remains enabled (because `isDirty` is still `true`), and the user can retry.

## Tasks / Subtasks

- [ ] **Add dirty-tracking state to `SectionEditor.vue`** (AC: #1, #2)
  - [ ] Open `agent-diva-gui/src/components/persona-memory/SectionEditor.vue`
  - [ ] Add reactive state:
        ```ts
        const originalContent = ref<string>(props.initialContent ?? '');
        const draftContent = ref<string>(props.initialContent ?? '');
        const saving = ref(false);
        const saveError = ref<string | null>(null);
        ```
  - [ ] Add computed dirty flag:
        ```ts
        const isDirty = computed(() => draftContent.value !== originalContent.value);
        ```
  - [ ] Watch `props.initialContent` and reset `originalContent` / `draftContent` / `saveError` whenever the parent loads a new section.

- [ ] **Wire textarea input to `draftContent`** (AC: #1, #2)
  - [ ] Bind the Markdown textarea to `draftContent` via `v-model`.
  - [ ] Ensure typing updates `draftContent` immediately so `isDirty` flips to `true`.
  - [ ] Keep the preview pane re-rendered from `draftContent` (Story 3.1 owns the preview; this story only requires the same reactive source).

- [ ] **Implement Save button enable/disable logic** (AC: #1, #2, #5)
  - [ ] Add a primary Save button in the editor toolbar.
  - [ ] Bind disabled state to:
        ```ts
        :disabled="!isDirty || saving"
        ```
  - [ ] When `saving` is `true`, render a small inline spinner inside the button and replace the label with `t('laputa.saving')`.

- [ ] **Add overwrite confirmation before saving** (AC: #3)
  - [ ] Import `appConfirm` from `agent-diva-gui/src/utils/appDialog.ts`.
  - [ ] In the Save click handler, if `originalContent.value.trim().length > 0`, await:
        ```ts
        const confirmed = await appConfirm(
          t('laputa.confirmSave.message', { section: props.displayName }),
          {
            title: t('laputa.confirmSave.title'),
            confirmLabel: t('laputa.confirmSave.confirm'),
            cancelLabel: t('laputa.confirmSave.cancel'),
          },
        );
        if (!confirmed) return;
        ```
  - [ ] If `originalContent` is empty, skip confirmation and save directly.

- [ ] **Implement the save handler in `SectionEditor.vue`** (AC: #4, #6, #7)
  - [ ] On Save click:
        ```ts
        saving.value = true;
        saveError.value = null;
        try {
          await writeLaputaSection(props.sectionName, draftContent.value);
          originalContent.value = draftContent.value;
          emit('saved', props.sectionName);
        } catch (err) {
          saveError.value = formatError(err);
        } finally {
          saving.value = false;
        }
        ```
  - [ ] Emit `saved` so `PersonaMemoryView.vue` can refresh the snapshot / last-updated time and show the success toast (handled in Story 3.3).
  - [ ] Do **not** reset `draftContent` on error so the user's edits are preserved.

- [ ] **Consume `SectionEditor.vue` events in `PersonaMemoryView.vue`** (AC: #6, #7)
  - [ ] Open `agent-diva-gui/src/components/PersonaMemoryView.vue`
  - [ ] Pass the loaded section content into `SectionEditor` as `initialContent`.
  - [ ] Listen for `@saved` and refresh the current section via `getLaputaSection(name)` (Story 3.3 owns the full refresh + toast flow; this story only emits the event).
  - [ ] Surface `saveError` in the editor or page-level error area.

- [ ] **Add / verify i18n keys** (AC: #3, #5)
  - [ ] Add to `agent-diva-gui/src/locales/zh.ts` and `agent-diva-gui/src/locales/en.ts` under the `laputa.*` namespace:
        - `laputa.save`
        - `laputa.saving`
        - `laputa.saved`
        - `laputa.confirmSave.title`
        - `laputa.confirmSave.message`
        - `laputa.confirmSave.confirm`
        - `laputa.confirmSave.cancel`
        - `laputa.saveFailed`

- [ ] **Write component-level tests** (AC: #1–#7)
  - [ ] Open or create `agent-diva-gui/src/components/persona-memory/__tests__/SectionEditor.spec.ts` (or equivalent Vitest location).
  - [ ] Mount `SectionEditor.vue` with `initialContent = 'original'`.
  - [ ] Type into the textarea and assert Save button becomes enabled.
  - [ ] Revert text to `'original'` and assert Save button becomes disabled.
  - [ ] Click Save with non-empty `originalContent` and assert `appConfirm` is called.
  - [ ] Confirm the dialog and assert `writeLaputaSection` is called with the current `draftContent`.
  - [ ] Mock a successful save and assert `isDirty` resets (button disabled).
  - [ ] Mock a failed save and assert the error is displayed and `draftContent` is preserved.
  - [ ] Assert that while saving, the button is disabled and shows the loading label.

- [ ] **Run validation gates**
  - [ ] `pnpm --filter agent-diva-gui test:unit` (or equivalent `vitest` command)
  - [ ] `just fmt-check`
  - [ ] `just check`
  - [ ] If `agent-diva-gui` changes are present, run the GUI smoke test recipe and record observation points.

## Dev Notes

### Relevant architecture patterns and constraints

- **Dirty detection is local and value-based**. `isDirty` must be a computed comparison between `draftContent` and `originalContent`. Do not maintain a separate mutable dirty flag that can drift out of sync.
- **Save must be idempotent-guarded**. Disable the Save button and set `saving` while the async call is in flight to prevent double-clicks or keyboard re-submission.
- **Overwrite confirmation only for existing content**. First-time writes (empty `originalContent`) should not ask for confirmation; this matches the assumption in `EXPERIENCE.md` and avoids an unnecessary friction point.
- **Error recovery preserves draft**. On failure, leave `draftContent` untouched so the user can inspect the error and retry without retyping.
- **Do not call the backend directly from the Vue component**. All persistence goes through `writeLaputaSection` in `agent-diva-gui/src/api/desktop.ts`, which proxies to the Tauri command and ultimately to `POST /api/laputa/section/:name/write`.
- **Keep proposal governance in the backend**. The frontend does not construct `EvolutionProposal` objects; it only sends the raw Markdown content.

### Source tree components to touch

- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` — add state, textarea binding, Save button, confirmation, save handler
- `agent-diva-gui/src/components/PersonaMemoryView.vue` — pass props, handle `@saved`, refresh content
- `agent-diva-gui/src/utils/appDialog.ts` — import `appConfirm` (read-only)
- `agent-diva-gui/src/api/desktop.ts` — import `writeLaputaSection` (read-only; wrapper added in Story 1.4)
  - `agent-diva-gui/src/locales/zh.ts` and `en.ts` — add `laputa.save`, `laputa.saving`, `laputa.saved`, `laputa.confirmSave.*`, `laputa.saveFailed`
- `agent-diva-gui/src/components/persona-memory/__tests__/SectionEditor.spec.ts` (or equivalent) — new unit tests

### State shape

```ts
// SectionEditor.vue
const props = defineProps<{
  sectionName: string;
  displayName: string;
  initialContent: string;
}>();

const emit = defineEmits<{
  (e: 'saved', sectionName: string): void;
}>();

const originalContent = ref<string>(props.initialContent ?? '');
const draftContent = ref<string>(props.initialContent ?? '');
const saving = ref(false);
const saveError = ref<string | null>(null);

const isDirty = computed(() => draftContent.value !== originalContent.value);

watch(
  () => props.initialContent,
  (next) => {
    originalContent.value = next ?? '';
    draftContent.value = next ?? '';
    saveError.value = null;
  },
);

async function handleSave() {
  if (!isDirty.value || saving.value) return;

  if (originalContent.value.trim().length > 0) {
    const confirmed = await appConfirm(
      t('laputa.confirmSave.message', { section: props.displayName }),
      {
        title: t('laputa.confirmSave.title'),
        confirmLabel: t('laputa.confirmSave.confirm'),
        cancelLabel: t('laputa.confirmSave.cancel'),
      },
    );
    if (!confirmed) return;
  }

  saving.value = true;
  saveError.value = null;
  try {
    await writeLaputaSection(props.sectionName, draftContent.value);
    originalContent.value = draftContent.value;
    emit('saved', props.sectionName);
  } catch (err) {
    saveError.value = (err as Error)?.message ?? String(err);
  } finally {
    saving.value = false;
  }
}
```

### Button template snippet

```vue
<button
  type="button"
  class="btn-primary"
  :disabled="!isDirty || saving"
  @click="handleSave"
>
  <span v-if="saving" class="spinner" />
  {{ saving ? t('laputa.saving') : t('laputa.save') }}
</button>
```

### Reference patterns

- **Dirty pattern**: `agent-diva-gui/src/components/settings/SelfEvolutionSettings.vue` uses a local draft object compared against an initial snapshot to derive whether settings have changed. Reuse the same value-comparison approach; for this story the comparison is simply `draftContent !== originalContent`.
- **Save pattern**: `agent-diva-gui/src/components/console/ConfigEditor.vue` shows a loading state, disables the Save button during the async call, surfaces errors inline, and keeps the editor content intact on failure. Mirror that exact behavior here.
- **Confirmation pattern**: `appConfirm` from `agent-diva-gui/src/utils/appDialog.ts` is used elsewhere for destructive actions; use the same shape (`{ title, message }`) and await the boolean result.

### UX / design constraints

- Save button styling follows `DESIGN.md` primary button token (`var(--accent)` background, white text, `var(--radius-sm)` radius).
- Disabled state uses `opacity: 0.5; cursor: not-allowed;` per `DESIGN.md`.
- Error text uses `var(--danger)` and is accompanied by an icon + message (not color alone).
- Confirmation title/message must be present in both `locales/zh.ts` and `locales/en.ts`.

### Testing standards summary

- Use `@vue/test-utils` and `vitest` patterns already present in `agent-diva-gui`.
- Mock `writeLaputaSection` and `appConfirm` at the module level so tests are deterministic and fast.
- Assert on button `disabled` attribute and label text.
- Assert that `writeLaputaSection` receives `props.sectionName` and the final `draftContent`.
- Assert that successful save resets dirty by checking the button disabled state after resolving the promise.

### Project Structure Notes

- This story builds on Story 3.1 (Markdown editor with preview) and Story 1.4 (`writeLaputaSection` in `desktop.ts`). It should not reimplement the preview pane or the API wrapper.
- Story 3.3 will add the post-save refresh, toast notification, and snapshot update. This story only needs to emit `@saved` so Story 3.3 can react.
- Story 3.5 will add the section-switch discard confirmation. This story focuses on the Save flow confirmation, not the switch flow.

### References

- `writeLaputaSection` wrapper: [Source: agent-diva-gui/src/api/desktop.ts]
- `appConfirm` dialog helper: [Source: agent-diva-gui/src/utils/appDialog.ts]
- Dirty comparison pattern: [Source: agent-diva-gui/src/components/settings/SelfEvolutionSettings.vue]
- Save + loading + error pattern: [Source: agent-diva-gui/src/components/console/ConfigEditor.vue]
- API contract and state flow: [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md §5]
- Save flow UX assumptions: [Source: docs/ux/persona-memory-laputa-2026-07-05/EXPERIENCE.md §State Patterns / Save flow]
- PRD: [Source: docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md §FR-104]
- Epic source: [Source: _bmad-output/planning-artifacts/epics.md §Epic 3 / Story 3.2]

## Dev Agent Record

### Agent Model Used

(To be filled during implementation)

### Debug Log References

(To be filled during implementation)

### Completion Notes List

- [ ] `SectionEditor.vue` state (`originalContent`, `draftContent`, `saving`, `saveError`, `isDirty`) added
- [ ] Textarea bound to `draftContent` and Save button disabled when `!isDirty || saving`
- [ ] Overwrite confirmation shown via `appConfirm` only for non-empty `originalContent`
- [ ] `writeLaputaSection(name, draftContent)` called on save
- [ ] Success resets `originalContent` and emits `@saved`; failure preserves `draftContent`
- [ ] i18n keys added to `zh.ts` and `en.ts`
- [ ] Component tests added and passing
- [ ] `just fmt-check && just check` clean

### File List

- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue`
- `agent-diva-gui/src/components/PersonaMemoryView.vue`
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src/locales/en.ts`
- `agent-diva-gui/src/components/persona-memory/__tests__/SectionEditor.spec.ts` (or equivalent test location)
