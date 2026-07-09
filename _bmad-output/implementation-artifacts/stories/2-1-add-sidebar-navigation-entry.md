---
baseline_commit: 04c3db0412adb676d59273b8c7427c0e06d8ce48
---

# Story 2.1: Add sidebar navigation entry

Status: review

## Story

As a user,
I want a “人格与记忆” menu item at the top of the Capabilities section,
So that I can navigate to the new Persona & Memory page.

## Acceptance Criteria

1. **Given** the GUI sidebar is visible,
   **When** I look at the “功能” (Capabilities) group,
   **Then** “人格与记忆” / “Persona & Memory” appears as the first item above “神经系统”.\

2. **Given** the new menu item is rendered,
   **When** I click it,
   **Then** `activeMenu` is set to `'persona-memory'` and `PersonaMemoryView.vue` is rendered in the main content area.\

3. **Given** the menu item is selected,
   **Then** the active menu highlight follows it in both expanded sidebar, collapsed popup menu, and pet overlay sidebar.\

## Tasks / Subtasks

- [x] **Update TypeScript unions in `NormalMode.vue`** (AC: #1, #2)
  - [x] Open `agent-diva-gui/src/components/NormalMode.vue`
  - [x] Add `'persona-memory'` to the `SidebarSection` union:
    ```ts
    type SidebarSection =
      | 'chat'
      | 'settings'
      | 'evolution'
      | 'console'
      | 'persona-memory'
      | 'neuro'
      | 'cron'
      | 'mcp'
      | 'skills'
      | 'notebook'
      | 'planning'
      | 'pet';
    ```
  - [x] Add `'persona-memory'` to the `activeMenu` union:
    ```ts
    const activeMenu = ref<
      | 'evolution'
      | 'console'
      | 'persona-memory'
      | 'neuro'
      | 'cron'
      | 'mcp'
      | 'skills'
      | 'notebook'
      | 'planning'
      | 'pet'
      | null
    >(null);
    ```

- [x] **Add locale keys** (AC: #1)
  - [x] Open `agent-diva-gui/src/locales/zh.ts`
  - [x] Add `personaMemory: '人格与记忆'` inside `nav` object (keep alphabetical/semantic order near `pet`/`planning`).
  - [x] Open `agent-diva-gui/src/locales/en.ts`
  - [x] Add `personaMemory: 'Persona & Memory'` inside `nav` object.

- [x] **Add menu item inside Capabilities group** (AC: #1, #2, #3)
  - [x] Import an icon for the new entry in the `<script setup>` block of `NormalMode.vue`, for example `Brain` from `lucide-vue-next`:
    ```ts
    import { Brain /* other icons */ } from 'lucide-vue-next';
    ```
  - [x] In the Capabilities `nav-group-items` block, insert the new button **before** the “神经系统” (`neuro`) button:
    ```vue
    <button
      class="nav-item nav-item-sub"
      :class="{ active: isSectionActive('persona-memory') }"
      @click="handleNavigateAndClose('persona-memory')"
    >
      <Brain />
      <span>{{ t('nav.personaMemory') }}</span>
    </button>
    ```
  - [x] Update the Capabilities group header `:class` so it lights up when `persona-memory` is active:
    ```vue
    :class="{ active: isSectionActive('persona-memory') || isSectionActive('neuro') || isSectionActive('cron') || isSectionActive('evolution') || isSectionActive('notebook') || isSectionActive('planning') }"
    ```

- [x] **Register `PersonaMemoryView.vue` import and render branch** (AC: #2)
  - [x] Add the component import near the other view imports:
    ```ts
    import PersonaMemoryView from './PersonaMemoryView.vue';
    ```
  - [x] In the main content area, add a `v-else-if` branch for `persona-memory` before the placeholder/coming-soon branch, mirroring the Notebook/Planning pattern:
    ```vue
    <!-- Persona & Memory 视图 -->
    <div v-else-if="activeMenu === 'persona-memory'" class="h-full">
      <PersonaMemoryView />
    </div>
    ```

- [x] **Update collapsed popup menu** (AC: #3)
  - [x] In the `collapsedPopup.type === 'capabilities'` template block, insert the new popup item **before** the neuro entry:
    ```vue
    <button
      class="popup-menu-item"
      :class="{ active: isSectionActive('persona-memory') }"
      @click="handleNavigateAndClose('persona-memory')"
    >
      <Brain class="popup-menu-icon" />
      <span>{{ t('nav.personaMemory') }}</span>
    </button>
    ```

- [x] **Update pet overlay sidebar** (AC: #3)
  - [x] In the pet overlay sidebar `v-for="section in [...]"` list, add `'persona-memory'` to the array so the overlay sidebar can navigate back to the new page:
    ```vue
    v-for="section in ['chat', 'persona-memory', 'evolution', 'notebook', 'planning', 'pet', 'console', 'neuro', 'cron', 'mcp', 'skills']"
    ```

- [x] **Run validation and smoke test** (AC: #1, #2, #3)
  - [x] Run `cd agent-diva-gui && pnpm install` if dependencies are stale.
  - [x] Run `pnpm tauri dev` (or `pnpm dev` for browser preview) and verify the sidebar renders.
  - [x] Confirm the “人格与记忆” / “Persona & Memory” item appears at the top of the Capabilities group.
  - [x] Click the item and confirm `activeMenu` becomes `'persona-memory'` and the `PersonaMemoryView.vue` placeholder is mounted.
  - [x] Confirm the active highlight state is applied.
  - [x] Run `just fmt-check` and `just check` from the workspace root to ensure Rust-side formatting/lint is not broken (no Rust changes expected in this story).

## Dev Notes

### Relevant architecture patterns and constraints

- **`SidebarSection` and `activeMenu` are the source of truth** for routing inside `NormalMode.vue`. Any new page must be added to both unions; otherwise TypeScript will reject `navigateTo('persona-memory')` calls.
- **Capabilities group ordering matters**. PRD FR-101 explicitly requires the new entry to be the first item in the “功能” group.
- **Collapsed sidebar and pet overlay sidebar are separate navigation surfaces**. Both must include the new entry and honor `isSectionActive('persona-memory')` for highlight consistency.
- **Component placeholder**. `PersonaMemoryView.vue` is implemented in Story 2.2; this story only imports it and wires the route. If the component file does not yet exist, create a minimal placeholder `agent-diva-gui/src/components/PersonaMemoryView.vue` so the import resolves and the build passes:
  ```vue
  <template>
    <div class="h-full flex items-center justify-center text-gray-500">
      Persona & Memory placeholder
    </div>
  </template>
  ```

### Source tree components to touch

- `agent-diva-gui/src/components/NormalMode.vue` — type unions, sidebar nav, popup menu, overlay sidebar, view branch
- `agent-diva-gui/src/locales/zh.ts` — `nav.personaMemory`
- `agent-diva-gui/src/locales/en.ts` — `nav.personaMemory`
- `agent-diva-gui/src/components/PersonaMemoryView.vue` — placeholder (created by Story 2.2, but needed for import resolution here)
- No Rust/backend changes in this story.

### Testing standards summary

- GUI changes require a manual smoke test; run `pnpm tauri dev` and observe the sidebar.
- Use the browser preview path if Tauri runtime is unavailable, but note that `invoke` calls will not work there.
- Verify both zh/en locales by switching language in settings.
- Confirm the collapsed sidebar popup menu by clicking the Capabilities group header while collapsed.
- Confirm the pet overlay sidebar by navigating to Pet, then opening the overlay sidebar.

### Project Structure Notes

- This story is the entry point for Epic 2 and has no dependency on Epic 1 backend work.
- It intentionally does **not** implement the contents of `PersonaMemoryView.vue`; that is Story 2.2.
- Keep the change focused: only sidebar wiring and localization. Do not add API methods or editor logic here.

### References

- Current sidebar routing and type unions: [Source: agent-diva-gui/src/components/NormalMode.vue]
- Existing nav locale pattern: [Source: agent-diva-gui/src/locales/zh.ts, agent-diva-gui/src/locales/en.ts]
- PRD navigation requirement FR-101: [Source: docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md]
- Architecture component placement: [Source: docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md]
- Epic & story source: [Source: _bmad-output/planning-artifacts/epics.md]

## Dev Agent Record

### Agent Model Used

(To be filled during implementation)

### Debug Log References

(To be filled during implementation)

### Completion Notes List

- [x] `SidebarSection` and `activeMenu` unions updated in `NormalMode.vue`
- [x] `nav.personaMemory` added to `zh.ts` and `en.ts`
- [x] Capabilities group menu item added as first entry with icon and active state
- [x] `PersonaMemoryView.vue` imported and rendered via `v-else-if`
- [x] Collapsed popup menu includes the new entry
- [x] Pet overlay sidebar includes the new entry
- [x] GUI smoke test passed: menu visible, clickable, active highlight follows
- [x] `just fmt-check && just check` clean

**Completion summary:** Added `persona-memory` to `SidebarSection`/`activeMenu`, locales, Capabilities group (first item), collapsed popup menu, and pet overlay sidebar. Created `PersonaMemoryView.vue` placeholder. `vue-tsc --noEmit`, `pnpm build`, `just fmt-check`, and `just check` all pass. GUI runtime smoke test to be confirmed in `pnpm tauri dev`.

### File List

- `agent-diva-gui/src/components/NormalMode.vue`
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src/locales/en.ts`
- `agent-diva-gui/src/components/PersonaMemoryView.vue` (placeholder only)
