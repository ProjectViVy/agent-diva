# Story 2.4: View section content

Status: ready-for-dev

## Story

As a user,
I want to see the current Markdown content of a selected section,
So that I know what is stored before editing.

## Acceptance Criteria

1. **Given** a section is selected,
   **When** its content loads successfully,
   **Then** the right panel shows the section title, status badge, last updated time, and Markdown content.

2. **Given** the initial implementation,
   **When** the editor renders,
   **Then** it uses a read-only rendering or simple text display of the section Markdown (full textarea + preview split is deferred to Story 3.1).

3. **Given** the user switches sections in a local-file scenario,
   **When** the new section content loads,
   **Then** the right panel refreshes within 200ms.

4. **Given** the section content is empty or the section does not exist,
   **When** the editor renders,
   **Then** it displays the appropriate empty-state title and description using `laputa.emptyTitle`, `laputa.emptyDesc`, `laputa.uninitializedTitle`, or `laputa.uninitializedDesc`.

5. **Given** the section content is loading,
   **When** the editor renders,
   **Then** it shows a loading skeleton instead of stale or blank content.

6. **Given** the section read fails,
   **When** the error is returned,
   **Then** the editor shows an error state with a retry button and preserves the previously selected section context.

## Tasks / Subtasks

- [ ] **Create `SectionEditor.vue` read-only shell** (AC: #1, #2)
  - [ ] Create `agent-diva-gui/src/components/persona-memory/SectionEditor.vue`
  - [ ] Define props:
        ```ts
        interface Props {
          sectionName: LaputaSectionName;
          content?: string;
          lastUpdated?: string;
          status?: 'owned' | 'tbd' | string;
          loading?: boolean;
          error?: string;
        }
        const props = withDefaults(defineProps<Props>(), {
          content: '',
          lastUpdated: '',
          status: 'tbd',
          loading: false,
          error: '',
        });
        ```
  - [ ] Emit `retry` when the user clicks the retry button.

- [ ] **Add Markdown rendering with `markdown-it` + `highlight.js`** (AC: #1, #2)
  - [ ] Import `MarkdownIt` and `hljs` the same way as `NotebookView.vue` and `ChatView.vue`:
        ```ts
        import MarkdownIt from 'markdown-it';
        import hljs from 'highlight.js';
        import 'highlight.js/styles/github-dark.css';
        ```
  - [ ] Configure the renderer with fenced code highlighting:
        ```ts
        const md = new MarkdownIt({
          html: false,
          linkify: true,
          highlight(str: string, lang: string) {
            if (lang && hljs.getLanguage(lang)) {
              try {
                return hljs.highlight(str, { language: lang }).value;
              } catch {
                // fall through
              }
            }
            return '<pre><code>' + str.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;') + '</code></pre>';
          },
        });
        ```
  - [ ] Compute `renderedContent = computed(() => md.render(props.content ?? ''))`.

- [ ] **Build the toolbar** (AC: #1)
  - [ ] Left side: localized section title, status badge (`owned`/`tbd`), and last updated time.
  - [ ] Right side: placeholder Secondary Button for "History" (disabled or hidden in this story; wired in Story 3.4) and placeholder Primary Button for "Save" (disabled in this story; wired in Story 3.2).
  - [ ] Use `t('laputa.status.owned')` / `t('laputa.status.tbd')` for badge text.

- [ ] **Implement loading, empty, error, and content states** (AC: #4, #5, #6)
  - [ ] Loading state: render at least a title skeleton and several `.skeleton-line` elements with `skeleton-pulse` animation.
  - [ ] Empty state (section exists but content is empty): show `laputa.emptyTitle` + `laputa.emptyDesc`.
  - [ ] Uninitialized state (`.laputa/` missing or section not found): show `laputa.uninitializedTitle` + `laputa.uninitializedDesc`.
  - [ ] Error state: show `laputa.loadError`, the backend message, and a "Retry" Secondary Button that emits `retry`.
  - [ ] Content state: render the Markdown HTML inside a `.markdown-body` container.

- [ ] **Style with project CSS variables** (AC: #1)
  - [ ] Use `var(--panel)`, `var(--panel-solid)`, `var(--line)`, `var(--text)`, `var(--text-muted)`, `var(--accent)`, `var(--accent-bg-light)`, `var(--accent-border)`, `var(--radius)`, `var(--radius-sm)`, and `--danger`.
  - [ ] Match spacing and typography tokens from `DESIGN.md`:
    - Toolbar padding: `12px 16px`
    - Content padding: `16px–24px`
    - Section title: `0.875rem` / 500
    - Last updated: `0.75rem` / 400 / `var(--text-muted)`
    - Badge: pill shape, uppercase, `0.625rem` / 600
  - [ ] Reuse the same `.markdown-body` deep selectors used in `NotebookView.vue` for headings, lists, code blocks, blockquotes, and tables.

- [ ] **Integrate `SectionEditor.vue` into `PersonaMemoryView.vue`** (AC: #1, #3)
  - [ ] Open `agent-diva-gui/src/components/PersonaMemoryView.vue` (created in Story 2.2)
  - [ ] Import `SectionEditor` and place it in the right panel.
  - [ ] Pass `sectionName`, `content`, `lastUpdated`, `status`, `loading`, and `error` as props.
  - [ ] Call `getLaputaSection(name)` from `desktop.ts` when the selected section changes.
  - [ ] Call `getLaputaSnapshot()` from `desktop.ts` on mount to obtain `last_modified` / `status` metadata for the selected section.
  - [ ] Measure section-switch time locally and ensure it stays under 200ms for local-file scenarios.

- [ ] **Add i18n keys** (AC: #4)
  - [ ] Add to `agent-diva-gui/src/locales/zh.ts` under `laputa`:
        ```ts
        loading: '正在加载 Laputa 数据…',
        emptyTitle: '此 section 还没有内容',
        emptyDesc: '在右侧编辑器中输入 Markdown 内容，然后点击保存。',
        uninitializedTitle: 'Laputa 尚未初始化',
        uninitializedDesc: '保存任意 section 后，系统将自动创建记忆骨架。',
        loadError: '无法加载记忆数据',
        retry: '重试',
        history: '历史',
        save: '保存',
        status: {
          owned: '已就绪',
          tbd: '待定',
        },
        ```
  - [ ] Add equivalent English keys to `agent-diva-gui/src/locales/en.ts`.

- [ ] **Add component-level tests** (AC: #1, #3)
  - [ ] Open or create `agent-diva-gui/src/components/persona-memory/__tests__/SectionEditor.spec.ts` (or equivalent Vitest test file)
  - [ ] Test that selecting a section with content renders the Markdown body.
  - [ ] Test that the toolbar shows the section title, status badge, and last updated time.
  - [ ] Test that the loading skeleton appears when `loading === true`.
  - [ ] Test that the error state appears when `error` is provided and emits `retry` on button click.
  - [ ] Test that the empty and uninitialized states render the correct i18n keys.

- [ ] **Run validation gates**
  - [ ] `pnpm test:unit` (or `vitest run`) for the new component tests
  - [ ] `pnpm type-check` (or `vue-tsc --noEmit`) for the GUI package
  - [ ] `just fmt-check`
  - [ ] `just check`
  - [ ] Manual smoke test: open "Persona & Memory", select `identity`, verify content renders; switch sections and measure load time locally.

## Dev Notes

### Relevant architecture patterns and constraints

- **Read-only in this story**: `SectionEditor.vue` is intentionally a view-only component for Story 2.4. The full Markdown textarea + preview editor, dirty tracking, save flow, and history modal are handled in Epic 3.
- **Props-driven component**: Keep `SectionEditor.vue` props-driven so `PersonaMemoryView.vue` owns loading/error/data fetching. This matches the `Fetch-Edit-Save ref 三元组` pattern described in the architecture spine.
- **No direct filesystem access**: This story only reads data through `getLaputaSection` and `getLaputaSnapshot` from `desktop.ts`. Writes are deferred to Epic 1 / Story 3.2.
- **Locale-driven display names**: Section display names come from `t('laputa.sections.<name>')`, not from the raw backend identifier.

### Source tree components to touch

- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` — new component (this story)
- `agent-diva-gui/src/components/PersonaMemoryView.vue` — integration (created in Story 2.2)
- `agent-diva-gui/src/api/desktop.ts` — consume `getLaputaSection` and `getLaputaSnapshot` (no changes if already exposed; verify)
- `agent-diva-gui/src/locales/zh.ts` — add `laputa.loading`, `laputa.emptyTitle`, `laputa.emptyDesc`, `laputa.uninitializedTitle`, `laputa.uninitializedDesc`, plus status/section keys as needed
- `agent-diva-gui/src/locales/en.ts` — English counterparts
- `agent-diva-gui/src/components/persona-memory/__tests__/SectionEditor.spec.ts` — new tests

### Testing standards summary

- Use Vitest + Vue Test Utils for component tests.
- Mock `vue-i18n` with the English locale so tests are stable regardless of default language.
- For the local switch-time assertion, use `performance.now()` around `getLaputaSection` calls in a manual smoke test; do not assert on absolute timing in CI unit tests.
- Validate that `renderedContent` is produced by `markdown-it` and contains expected HTML for the provided Markdown.

### Project Structure Notes

- This story belongs to Epic 2 (page skeleton and section browsing) and focuses on the right-panel content view.
- `SectionEditor.vue` will be extended in Epic 3 to include the textarea editor, preview pane, dirty tracking, save button, and history modal trigger.
- Keep the component path at `agent-diva-gui/src/components/persona-memory/SectionEditor.vue` to match the architecture spine.

### References

- Example story structure: [Source: _bmad-output/implementation-artifacts/stories/1-1-add-direct-edit-and-apply-to-laputa-service.md]
- PRD FR-103 / FR-107: [Source: docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md]
- Architecture spine component seed and state flow: [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md]
- Visual tokens and component specs: [Source: docs/ux/persona-memory-laputa-2026-07-05/DESIGN.md]
- Experience flows and i18n key suggestions: [Source: docs/ux/persona-memory-laputa-2026-07-05/EXPERIENCE.md]
- Epic breakdown: [Source: _bmad-output/planning-artifacts/epics.md]
- Markdown rendering reference: [Source: agent-diva-gui/src/components/NotebookView.vue]
- Alternative Markdown rendering reference: [Source: agent-diva-gui/src/components/ChatView.vue]
- Laputa API wrappers: [Source: agent-diva-gui/src/api/desktop.ts]

## Dev Agent Record

### Agent Model Used

(To be filled during implementation)

### Debug Log References

(To be filled during implementation)

### Completion Notes List

- [ ] `SectionEditor.vue` created with toolbar, Markdown rendering, loading/empty/error states
- [ ] `SectionEditor.vue` integrated into `PersonaMemoryView.vue`
- [ ] `getLaputaSection` and `getLaputaSnapshot` consumed from `desktop.ts`
- [ ] Required i18n keys added to both `zh.ts` and `en.ts`
- [ ] Component tests added and passing
- [ ] Local section-switch time measured at under 200ms
- [ ] `just fmt-check && just check` clean

### File List

- `agent-diva-gui/src/components/persona-memory/SectionEditor.vue`
- `agent-diva-gui/src/components/PersonaMemoryView.vue`
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src/locales/en.ts`
- `agent-diva-gui/src/components/persona-memory/__tests__/SectionEditor.spec.ts`

(End of file)
