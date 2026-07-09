# Story 3.1: Implement Markdown editor with preview

Status: review

## Story

As a user,
I want a Markdown textarea with a live preview pane,
So that I can edit section content and see how it renders.

## Acceptance Criteria

1. **Given** a section is selected,
   **When** the editor renders,
   **Then** it shows a textarea (min-height 320px, vertically resizable) on one side and a rendered Markdown preview on the other.

2. **Given** the user types Markdown in the textarea,
   **When** the input changes,
   **Then** the preview pane updates (debounced or reactively) to reflect the rendered output.

3. **Given** the preview pane renders,
   **When** it processes Markdown content,
   **Then** it uses `markdown-it` + `highlight.js`, consistent with `NotebookView.vue` and `ChatView.vue`.

4. **Given** the editor is displayed,
   **When** the active theme changes (love / dark / default / miku),
   **Then** the textarea and preview pane continue to use CSS variables from `styles.css` and remain theme-aware.

5. **Given** the viewport is wide (≥1024px),
   **When** the editor renders,
   **Then** the textarea and preview pane appear side-by-side.

6. **Given** the viewport is narrow (<1024px),
   **When** the editor renders,
   **Then** the editor switches to a stacked layout or tabbed edit/preview mode.

7. **Given** the component is integrated in `PersonaMemoryView.vue`,
   **When** the parent needs to render toolbar actions (save, history),
   **Then** `SectionEditor.vue` exposes a toolbar slot or integration point so the parent can place buttons without modifying the editor.

8. **Given** the component is used with `v-model`,
   **When** the textarea content changes,
   **Then** it emits `update:modelValue`, and on explicit save action it emits `save`.

## Tasks / Subtasks

- [x] **Create `SectionEditor.vue` with props, emits, and local state** (AC: #1, #8)
  - [x] Open `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` (create the directory if needed).
  - [x] Define props:
        ```ts
        const props = defineProps<{
          modelValue: string;
          sectionName: string;
        }>();
        ```
  - [x] Define emits:
        ```ts
        const emit = defineEmits<{
          (e: 'update:modelValue', value: string): void;
          (e: 'save'): void;
        }>();
        ```
  - [x] Keep a local `draft` ref that is initialized from `props.modelValue` and kept in sync via `watch(() => props.modelValue, ...)`. Emit `update:modelValue` on textarea input.

- [x] **Add Markdown renderer using `markdown-it` + `highlight.js`** (AC: #2, #3)
  - [x] Import `MarkdownIt` from `markdown-it`, `hljs` from `highlight.js`, and `highlight.js/styles/github-dark.css`.
  - [x] Create a module-level renderer mirroring `NotebookView.vue`:
        ```ts
        const md = new MarkdownIt({
          html: false,
          linkify: true,
          highlight(str: string, lang: string) {
            if (lang && hljs.getLanguage(lang)) {
              try {
                return hljs.highlight(str, { language: lang, ignoreIllegals: true }).value;
              } catch {
                // fall through
              }
            }
            return '<pre><code>' + str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;') + '</code></pre>';
          },
        });
        ```
  - [x] Compute `renderedHtml = computed(() => md.render(draft.value))`.

- [x] **Implement textarea with theme-aware styling** (AC: #1, #4)
  - [x] Add a `<textarea>` bound to `draft` via `v-model`.
  - [x] Apply CSS from `DESIGN.md` §Components → Markdown Editor:
        ```css
        .section-editor-textarea {
          width: 100%;
          min-height: 320px;
          padding: 1rem;
          border: 1px solid var(--line);
          border-radius: var(--radius-sm);
          background: var(--panel-solid);
          color: var(--text);
          font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
          font-size: 0.875rem;
          line-height: 1.6;
          resize: vertical;
          transition: border-color 0.15s ease, box-shadow 0.15s ease;
        }
        .section-editor-textarea:focus {
          outline: none;
          border-color: var(--accent);
          box-shadow: 0 0 0 2px var(--accent-glow);
        }
        ```
  - [x] Ensure the textarea is keyboard-focusable and has an `aria-label` such as `aria-label="Laputa section editor"`.

- [x] **Implement preview pane with theme-aware styling** (AC: #1, #2, #3, #4)
  - [x] Add a preview container that renders `renderedHtml` with `v-html`.
  - [x] Apply CSS from `DESIGN.md` §Components → Preview Pane:
        ```css
        .section-editor-preview {
          background: var(--panel);
          border: 1px solid var(--line);
          border-radius: var(--radius-sm);
          padding: 1rem;
          overflow-y: auto;
          min-height: 320px;
        }
        ```
  - [x] Add scoped `:deep()` selectors for rendered Markdown elements (headings, lists, code, blockquote, links, tables) so they inherit theme colors, similar to `NotebookView.vue` / `ChatView.vue`.

- [x] **Add responsive layout** (AC: #5, #6)
  - [x] Use a flex container with `flex-col` below `1024px` and `flex-row` at `1024px` and above.
  - [x] In wide mode, assign `flex: 1 1 0` (or `w-1/2`) to both textarea and preview panes, with a `min-width: 0` and `gap: 1rem`.
  - [x] In narrow mode, stack the textarea above the preview pane. Optionally render tabs (“Edit” / “Preview”) to save vertical space; if tabbed, default to the Edit tab.

- [x] **Add toolbar integration point** (AC: #7)
  - [x] Reserve a toolbar area at the top of `SectionEditor.vue` using a named slot:
        ```vue
        <div class="section-editor-toolbar">
          <div class="section-editor-toolbar-title">
            <span>{{ sectionName }}</span>
          </div>
          <div class="section-editor-toolbar-actions">
            <slot name="toolbar-actions" />
          </div>
        </div>
        ```
  - [x] Do not place Save/History buttons inside `SectionEditor.vue`; the parent `PersonaMemoryView.vue` will provide them via the slot (covered in Story 3.2).

- [x] **Add manual smoke test** (AC: #2)
  - [x] Run the GUI with `pnpm tauri dev`.
  - [x] Navigate to “人格与记忆”, select `identity`, and type the following into the textarea:
        ```markdown
        # Identity

        - Kind: assistant
        - Tone: warm

        ```rust
        fn hello() {}
        ```
        ```
  - [x] Verify the preview pane updates to show a heading, bullet list, and highlighted Rust code block.

## Dev Notes

### Relevant architecture patterns and constraints

- **No direct file writes in GUI**. `SectionEditor.vue` only emits events; persistence is handled by the parent through `writeLaputaSection` (Story 1.4 / Epic 1).
- **Theme consistency is mandatory**. Every color, border, radius, and shadow must come from `agent-diva-gui/src/styles.css` CSS variables. Do not hard-code colors.
- **Markdown renderer consistency**. Use the same `markdown-it` + `highlight.js` setup already used in `NotebookView.vue` and `ChatView.vue` so users see the same rendering behavior across the app.
- **Security**. Keep `html: false` in `MarkdownIt` options to avoid XSS from user-edited section content.
- **Reactive vs. debounced preview**. A `computed` property (`renderedHtml`) is sufficient for local preview and avoids extra state. If typing feels sluggish on very large sections, introduce a `useDebounce` wrapper, but start with the simpler computed approach.

### Source tree components to touch

- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` — new component (this story)
- `agent-diva-gui/src/components/PersonaMemoryView.vue` — mount `SectionEditor.vue` and wire `v-model` / `save` (parent integration is the focus of Story 3.2; in this story, ensure the component contract supports it)
- `agent-diva-gui/src/locales/zh.ts` and `agent-diva-gui/src/locales/en.ts` — no new keys required for the editor itself; toolbar labels live in the parent

### Testing standards summary

- **Manual smoke test** is the primary verification for this GUI story because the component is mostly presentational.
- Verify that typing Markdown updates the preview within one frame/render cycle.
- Verify code blocks are syntax-highlighted (Rust/JSON samples from Laputa sections).
- Verify the textarea focus ring uses `--accent` and `--accent-glow`.
- Verify the layout switches from side-by-side to stacked when the window is narrowed below 1024px.
- If automated component tests exist or are added later, assert:
  - `update:modelValue` is emitted on input.
  - `save` is emitted when the save shortcut/button (in parent) triggers it.
  - `renderedHtml` matches `md.render(currentValue)`.

### Project Structure Notes

- This story is part of **Epic 3: Section 编辑、保存与变更历史**.
- It depends on:
  - Epic 1 (backend `writeLaputaSection` API) being available for the parent to call on `save`.
  - Epic 2 (page skeleton and `PersonaMemoryView.vue`) providing the container, selected section, and toolbar buttons.
- This story intentionally only builds the editor component. Save button state/dirty tracking, confirmation dialogs, and history modal are covered by Stories 3.2–3.5.

### References

- Markdown renderer pattern: [Source: agent-diva-gui/src/components/NotebookView.vue] and [Source: agent-diva-gui/src/components/ChatView.vue]
- Theme CSS variables: [Source: agent-diva-gui/src/styles.css]
- Visual design tokens: [Source: docs/ux/persona-memory-laputa-2026-07-05/DESIGN.md]
- Experience/behavior contract: [Source: docs/ux/persona-memory-laputa-2026-07-05/EXPERIENCE.md]
- Architecture spine: [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md]
- Epic source: [Source: _bmad-output/planning-artifacts/epics.md]

## Dev Agent Record

### Agent Model Used

kimi-for-coding

### Debug Log References

- Removed unused `selectedSectionMeta`, `sectionContentString`, and `saveError` refs from `PersonaMemoryView.vue` after switching to `v-model` contract.
- Reused existing `laputa.save` and `laputa.history` i18n keys for toolbar buttons.

### Completion Notes List

- [x] `SectionEditor.vue` created under `agent-diva-gui/src/components/persona-memory/`
- [x] Props `modelValue` and `sectionName`, emits `update:modelValue` and `save` implemented
- [x] Textarea styled with CSS variables, min-height 320px, vertical resize, focus ring
- [x] Preview pane uses `markdown-it` + `highlight.js` and updates on input
- [x] Side-by-side layout on wide screens, tabbed edit/preview on narrow screens
- [x] Toolbar slot/integration point added for parent-supplied Save/History buttons
- [x] `PersonaMemoryView.vue` wired with `v-model`, `@save`, and `#toolbar-actions` slot
- [x] Unit tests updated in `SectionEditor.spec.ts`
- [x] Validation passed: `pnpm vue-tsc --noEmit`, `pnpm build`, `pnpm test SectionEditor.spec.ts`, `just fmt-check`, `just check`

### File List

- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue`
- `agent-diva-gui/src/components/PersonaMemoryView.vue`
- `agent-diva-gui/src/components/persona-memory/__tests__/SectionEditor.spec.ts`
- `agent-diva/_bmad-output/implementation-artifacts/stories/3-1-implement-markdown-editor-with-preview.md`
