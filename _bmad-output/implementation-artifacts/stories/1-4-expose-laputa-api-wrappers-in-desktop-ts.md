# Story 1.4: Expose Laputa API wrappers in desktop.ts

Status: review

## Story

As a frontend developer,
I want `getLaputaSnapshot` and `writeLaputaSection` available in `desktop.ts`,
So that the new `PersonaMemoryView.vue` page can call Laputa APIs consistently with the rest of the GUI.

## Acceptance Criteria

1. **Given** `agent-diva-gui/src/api/desktop.ts`,
   **When** it is imported by `PersonaMemoryView.vue`,
   **Then** it exports `getLaputaSnapshot(since?)` wrapping `invoke('laputa_get_snapshot', { since })`
   **And** the function returns a typed `LaputaSnapshot` containing all 14 section summaries.

2. **Given** the same module,
   **When** the page calls `writeLaputaSection(name, content, summary?)`,
   **Then** it wraps `invoke('laputa_write_section', { name, content, summary })`
   **And** it uses the shared `LaputaSectionName` type for `name`
   **And** it forwards `summary` as `null` when omitted, matching the existing nullable argument pattern.

3. **Given** the new wrappers are added,
   **When** existing code imports `getLaputaSection`, `applyLaputaProposal`, etc.,
   **Then** those exports continue to work unchanged with the same signatures and return types.
   **And** `listLaputaChangelog` keeps its existing positional signature and return type, while also accepting an optional filters object for section/limit filtering (used by Story 3.4).

4. **Given** the module is type-checked,
   **When** `pnpm build` (which runs `vue-tsc --noEmit`) executes,
   **Then** no TypeScript errors are introduced by the new interfaces or wrappers.

5. **Given** the new wrappers are implemented,
   **When** the Vitest unit test in `src/api/desktop.laputa.test.ts` runs,
   **Then** it verifies that `getLaputaSnapshot` and `writeLaputaSection` call `invoke` with the correct Tauri command names and payloads.

## Tasks / Subtasks

- [x] **Add `LaputaSnapshot` and `WriteLaputaSectionResult` interfaces** (AC: #1, #2)
  - [x] Open `agent-diva-gui/src/api/desktop.ts`
  - [x] In the "Laputa / Evolution Governance API" section (after the existing `LaputaSection` interface), add:
        ```ts
        export interface LaputaSnapshot {
          schema_version: string;
          sections: Record<string, LaputaSection>;
          changed_sections: string[];
          updated_at?: string | null;
          server_time: string;
        }

        export interface WriteLaputaSectionResult {
          changelog_id: string;
          applied_at: string;
          status?: string;
        }
        ```
  - [x] Reuse the existing `LaputaSection` interface; do not duplicate its fields.

- [x] **Expose `getLaputaSnapshot`** (AC: #1)
  - [x] Add the wrapper next to the existing Laputa getters (e.g. after `getLaputaSection`):
        ```ts
        export const getLaputaSnapshot = (since?: string) =>
          invoke<LaputaSnapshot>("laputa_get_snapshot", { since: since ?? null });
        ```
  - [x] Note: The `laputa_get_snapshot` Tauri command already exists in `agent-diva-gui/src-tauri/src/commands.rs`; this task only exposes it on the frontend.

- [x] **Expose `writeLaputaSection`** (AC: #2)
  - [x] Add the wrapper next to the existing section helpers:
        ```ts
        export const writeLaputaSection = (
          name: LaputaSectionName,
          content: string,
          summary?: string,
        ) =>
          invoke<WriteLaputaSectionResult>("laputa_write_section", {
            name,
            content,
            summary: summary ?? null,
          });
        ```
  - [x] The `name` parameter must be typed as `LaputaSectionName` (already exported in this file).
  - [x] The function signature matches the architecture contract: `writeLaputaSection(name, content, summary?)`.

- [x] **Extend `listLaputaChangelog` to support section filtering** (enables Story 3.4)
  - [x] Open `agent-diva-gui/src/api/desktop.ts`
  - [x] Replace the existing `listLaputaChangelog` wrapper with a backward-compatible version that accepts either positional arguments or a filters object:
        ```ts
        export interface ListLaputaChangelogFilters {
          section?: LaputaSectionName;
          limit?: number;
          page?: number;
          proposalId?: string;
        }

        export const listLaputaChangelog = (
          pageOrFilters?: number | ListLaputaChangelogFilters,
          pageSize?: number,
          proposalId?: string,
        ) => {
          if (typeof pageOrFilters === 'object') {
            const { section, limit, page, proposalId: pid } = pageOrFilters;
            return invoke<ChangelogPage>("laputa_list_changelog", {
              target_section: section ?? null,
              pageSize: limit ?? null,
              page: page ?? null,
              proposalId: pid ?? null,
            });
          }
          return invoke<ChangelogPage>("laputa_list_changelog", {
            page: pageOrFilters ?? null,
            pageSize: pageSize ?? null,
            proposalId: proposalId ?? null,
          });
        };
        ```
  - [x] Confirm the Tauri command `laputa_list_changelog` forwards these fields as query params to `GET /api/laputa/changelog`. If the backend expects `target_section`, map `section` to `target_section` in the invoke payload.

- [x] **Add unit tests for the new wrappers** (AC: #5)
  - [x] Create `agent-diva-gui/src/api/desktop.laputa.test.ts` (adjacent to the module under test):
        ```ts
        import { describe, it, expect, vi, beforeEach } from 'vitest';
        import { getLaputaSnapshot, writeLaputaSection } from './desktop';

        const invoke = vi.fn();
        vi.mock('@tauri-apps/api/core', () => ({ invoke: (...args: unknown[]) => invoke(...args) }));

        beforeEach(() => {
          invoke.mockReset();
          invoke.mockResolvedValue({});
        });

        describe('getLaputaSnapshot', () => {
          it('calls laputa_get_snapshot with null when since is omitted', async () => {
            await getLaputaSnapshot();
            expect(invoke).toHaveBeenCalledWith('laputa_get_snapshot', { since: null });
          });

          it('forwards the since parameter', async () => {
            await getLaputaSnapshot('2026-07-05T00:00:00Z');
            expect(invoke).toHaveBeenCalledWith('laputa_get_snapshot', { since: '2026-07-05T00:00:00Z' });
          });
        });

        describe('writeLaputaSection', () => {
          it('calls laputa_write_section with name, content and null summary', async () => {
            await writeLaputaSection('identity', '# Identity\n\nHello', undefined);
            expect(invoke).toHaveBeenCalledWith('laputa_write_section', {
              name: 'identity',
              content: '# Identity\n\nHello',
              summary: null,
            });
          });

          it('forwards the optional summary', async () => {
            await writeLaputaSection('memory_md', '{"note":"x"}', 'Quick edit');
            expect(invoke).toHaveBeenCalledWith('laputa_write_section', {
              name: 'memory_md',
              content: '{"note":"x"}',
              summary: 'Quick edit',
            });
          });
        });
        ```
  - [x] Run the test file with `pnpm test src/api/desktop.laputa.test.ts`.

- [x] **Run validation gates** (AC: #3, #4)
  - [x] `cd agent-diva-gui && pnpm build` (type-check + noEmit)
  - [x] `cd agent-diva-gui && pnpm test`
  - [x] `just fmt-check` (Rust formatting is unaffected, but run it to keep the gate green)
  - [x] `just check` (ensures the Rust workspace still compiles)

## Dev Notes

### Relevant architecture patterns and constraints

- **The frontend is a pure consumer**. `desktop.ts` must not contain business logic for proposal governance, filesystem access, or authority checks. It only forwards typed arguments to the Tauri bridge.
- **Use the existing nullable-argument pattern**. Existing wrappers such as `listLaputaChangelog` and `listLaputaProposals` convert `undefined` to `null` with `?? null`; the new wrappers must do the same for `since` and `summary`.
- **Error handling is consistent with other wrappers**. Do not wrap `invoke` in a local `try/catch`. Let Tauri/HTTP errors propagate as rejected Promises so callers (e.g. `PersonaMemoryView.vue`) can display `error.message` through the existing toast/alert system.
- **Type names must match backend serialization**. `LaputaSnapshot` mirrors `agent-diva-laputa/src/service.rs::LaputaSnapshot`:
  - `sections` is an object keyed by section-name strings (the Rust side uses `BTreeMap<String, LaputaSection>`).
  - `changed_sections` is an array of section-name strings.
  - `server_time` is always present; `updated_at` may be `null`.
- **`WriteLaputaSectionResult` mirrors Story 1.2**. The manager endpoint returns `{ status: "ok", changelog_id, applied_at }`. Because the Tauri command uses `post_laputa_full_response`, the frontend receives the full manager body. Keep `status` optional in the TypeScript interface so the type remains useful even if the wrapper is later changed to extract only the result fields.

### Source tree components to touch

- `agent-diva-gui/src/api/desktop.ts` — add the two new interfaces and two new exports.
- `agent-diva-gui/src/api/desktop.laputa.test.ts` — add unit tests (new file).
- No changes to `agent-diva-laputa`, `agent-diva-manager`, `agent-diva-gui/src-tauri`, or any Vue components in this story.

### Testing standards summary

- Prefer a focused unit test that mocks `@tauri-apps/api/core` and asserts the exact `invoke` calls. This keeps the test fast and avoids requiring a running manager/Tauri backend.
- For manual verification, import the wrappers from a temporary Vue component or a browser console script and confirm:
  - `getLaputaSnapshot()` returns a `LaputaSnapshot` with 14 sections.
  - `writeLaputaSection('identity', '# Test', 'smoke test')` returns a `changelog_id`.
- Do not commit the temporary script; only commit the `.test.ts` file.

### Project Structure Notes

- This story depends on **Story 1.3** (Tauri command `laputa_write_section` registered in `generate_handler!`). If Story 1.3 is not yet merged, the unit test will still pass because `invoke` is mocked; however, integration smoke testing must wait until the command is available.
- This story feeds **Epic 2** (`PersonaMemoryView.vue` loads the snapshot) and **Epic 3** (save flow calls `writeLaputaSection`).
- The `laputa_get_snapshot` Tauri command is already present; exposing it in `desktop.ts` is the only work required for the read side.

### References

- Existing `desktop.ts` Laputa wrappers: [Source: agent-diva-gui/src/api/desktop.ts]
- `laputa_get_snapshot` Tauri command implementation: [Source: agent-diva-gui/src-tauri/src/commands.rs — `laputa_get_snapshot`]
- `laputa_write_section` Tauri command spec: [Source: _bmad-output/implementation-artifacts/stories/1-3-add-tauri-command-for-writing-section.md]
- Manager write endpoint contract: [Source: _bmad-output/implementation-artifacts/stories/1-2-add-manager-endpoint-for-writing-section.md]
- Backend snapshot type: [Source: agent-diva-laputa/src/service.rs — `LaputaSnapshot`]
- Architecture API contract: [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md §4]
- Epic source: [Source: _bmad-output/planning-artifacts/epics.md]

## Dev Agent Record

### Agent Model Used

kimi-for-coding

### Debug Log References

- `pnpm vue-tsc --noEmit` passed in `agent-diva-gui/`
- `pnpm test src/api/desktop.laputa.test.ts` passed (4/4)
- `just fmt-check` passed at workspace root
- `just check` passed at workspace root

### Completion Notes List

- [x] `LaputaSnapshot` and `WriteLaputaSectionResult` interfaces added to `desktop.ts`
- [x] `getLaputaSnapshot` export added and type-checks
- [x] `writeLaputaSection` export added and type-checks
- [x] Existing Laputa exports unchanged
- [x] `src/api/desktop.laputa.test.ts` added and passing
- [x] `pnpm build` and `pnpm test` clean in `agent-diva-gui`
- [x] `just fmt-check && just check` clean at workspace root

### File List

- `agent-diva-gui/src/api/desktop.ts`
- `agent-diva-gui/src/api/desktop.laputa.test.ts`
