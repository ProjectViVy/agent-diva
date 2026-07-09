# Story 2.3: Implement grouped section list

Status: review

baseline_commit: ecf523d1d0ccfa0bf94785c90f46e882bd74c067

## Story

As a user,
I want the 14 Laputa sections grouped into Persona, Memory, Periodic, and Indexes,
So that I can quickly find the section I want to edit.

## Acceptance Criteria

1. **Given** the snapshot has been loaded,
   **When** the section list renders,
   **Then** it displays 4 collapsible groups with the correct 14 sections:
   - **Persona**: `identity`, `relationship`, `commitment`, `preferences`
   - **Memory**: `memory_md`, `history_md`
   - **Periodic**: `daily`, `weekly`, `monthly`
   - **Indexes**: `journal_reflective`, `proposal_inbox`, `changelog`, `report_indexes`, `aaak_summaries`

2. **Given** a section item is rendered,
   **When** the user looks at it,
   **Then** it shows its localized display name, English key, and status badge (`owned` / `tbd`).

3. **Given** the component is first rendered,
   **When** the groups are initialized,
   **Then** all 4 groups default to fully expanded.

4. **Given** the user clicks a section item,
   **When** the click is handled,
   **Then** the component emits `select(sectionName)` with the clicked `LaputaSectionName`
   **And** the clicked item receives the active-section styling.

5. **Given** the user clicks a group header,
   **When** the click is handled,
   **Then** the group collapses or expands
   **And** the chevron icon rotates to reflect the new state.

6. **Given** a section is selected,
   **When** it becomes the active item,
   **Then** it uses the styling specified in UX-DR2:
   - 3px left accent border (`var(--accent)`),
   - `var(--panel-solid)` background,
   - `var(--accent)` text,
   - subtle glow shadow (`0 2px 8px var(--accent-glow)`).

7. **Given** a section status badge is rendered,
   **When** the status is `owned` or `tbd`,
   **Then** the badge styling follows UX-DR3:
   - `owned`: `var(--accent-bg-light)` background, `var(--accent)` text, `1px solid var(--accent-border)` border
   - `tbd`: transparent background, `var(--text-muted)` text, `1px solid var(--line)` border

8. **Given** the component is implemented,
   **When** `pnpm vue-tsc --noEmit` is run,
   **Then** `SectionGroupList.vue` type-checks cleanly and all imports resolve.

9. **Given** the component is wired into the page,
   **When** the user switches sections or the snapshot refreshes,
   **Then** the active highlight and status badges stay in sync with `props.selectedSection` and `props.snapshot`.

## Tasks / Subtasks

- [x] **Create `SectionGroupList.vue`** (AC: #1, #2, #3, #4, #5, #6, #7)
  - [x] Open `agent-diva-gui/src/components/persona-memory/SectionGroupList.vue` (new file).
  - [x] Use `<script setup lang="ts">` and Vue 3 Composition API.
  - [x] Import `ref`, `computed` from `vue` and `useI18n` from `vue-i18n`.
  - [x] Import icons from `lucide-vue-next`:
    - `ChevronDown`, `ChevronRight` for group headers
    - `FileText` for each section item (or a per-group icon if preferred)
  - [x] Import the `LaputaSectionName` type from `desktop.ts`:
    ```ts
    import type { LaputaSectionName } from '../../api/desktop';
    ```
  - [x] Define the local snapshot shape (or import it from `PersonaMemoryView.vue` if exported there):
    ```ts
    interface LaputaSnapshot {
      sections: Record<LaputaSectionName, {
        status: 'owned' | 'tbd';
        last_modified?: string | null;
      }>;
    }
    ```
  - [x] Define props and emit:
    ```ts
    interface Props {
      snapshot: LaputaSnapshot | null;
      selectedSection: LaputaSectionName;
    }
    const props = defineProps<Props>();
    const emit = defineEmits<{
      (e: 'select', sectionName: LaputaSectionName): void;
    }>();
    ```
  - [x] Define the static 4-group configuration as a constant:
    ```ts
    const GROUPS: { key: string; sections: LaputaSectionName[] }[] = [
      { key: 'persona', sections: ['identity', 'relationship', 'commitment', 'preferences'] },
      { key: 'memory', sections: ['memory_md', 'history_md'] },
      { key: 'periodic', sections: ['daily', 'weekly', 'monthly'] },
      { key: 'indexes', sections: ['journal_reflective', 'proposal_inbox', 'changelog', 'report_indexes', 'aaak_summaries'] },
    ];
    ```
  - [x] Initialize an `expanded` ref with every group set to `true`:
    ```ts
    const expanded = ref<Record<string, boolean>>({
      persona: true,
      memory: true,
      periodic: true,
      indexes: true,
    });
    ```
  - [x] Implement `toggleGroup(groupKey: string)` that flips `expanded.value[groupKey]`.
  - [x] Implement `selectSection(name: LaputaSectionName)` that emits `select(name)`.
  - [x] Implement `getSectionStatus(name: LaputaSectionName): 'owned' | 'tbd'` that reads `props.snapshot?.sections[name]?.status ?? 'tbd'`.
  - [x] Build the template:
    - One wrapper with `role="navigation"` and `aria-label="Laputa sections"`.
    - For each group, render a clickable header that toggles the group, shows the localized group name (`t('laputa.groups.' + group.key)`), and a chevron.
    - When expanded, render the section items.
    - Each item shows:
      - `FileText` icon
      - localized section name (`t('laputa.sections.' + section)`)
      - English key as muted helper text
      - status badge (`t('laputa.status.' + status)`)
    - Apply `.section-item--active` when `section === props.selectedSection`.
  - [x] Add scoped CSS using `agent-diva-gui/src/styles.css` variables:
    - Section item padding, border-radius, hover background from `var(--accent-bg-light)`.
    - Active state exactly as specified in AC #6.
    - Status badge pill shape, uppercase, `0.625rem` / 600.
    - Group header styling consistent with `DESIGN.md` group title tokens.
  - [x] Add accessibility attributes:
    - Group header buttons have `aria-expanded` matching the expanded state.
    - Section item buttons have `aria-current="true"` when active.
    - Chevron icons are decorative (`aria-hidden="true"`).

- [x] **Add `laputa.*` i18n keys** (AC: #2, #7)
  - [x] In `agent-diva-gui/src/locales/zh.ts`, add or extend the `laputa` namespace with:
    ```ts
    laputa: {
      // ...existing keys from Story 2.2/2.4...
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
    }
    ```
  - [x] In `agent-diva-gui/src/locales/en.ts`, add the English equivalents:
    ```ts
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
    ```

- [x] **Wire `SectionGroupList.vue` into `PersonaMemoryView.vue`** (AC: #4, #9)
  - [x] Open `agent-diva-gui/src/components/PersonaMemoryView.vue` (created in Story 2.2).
  - [x] Ensure `SectionGroupList` is imported.
  - [x] Bind the component in the left panel with:
    ```vue
    <SectionGroupList
      :snapshot="snapshot"
      :selected-section="selectedSection"
      @select="onSectionSelect"
    />
    ```
  - [x] If the placeholder from Story 2.2 used a different prop name (e.g., `:selected`), rename it to `:selected-section` to match this component.
  - [x] Verify that `onSectionSelect(name: LaputaSectionName)` updates `selectedSection` and triggers `loadSection(name)`.

- [x] **Smoke-test the grouped list** (AC: #1, #2, #3, #4, #5)
  - [x] Temporarily mount `SectionGroupList.vue` inside `PersonaMemoryView.vue` with a sample snapshot containing all 14 sections (mix of `owned` and `tbd`).
  - [x] Verify that exactly 14 section rows are rendered.
  - [x] Verify that the 4 group headers are rendered and all groups start expanded.
  - [x] Click a group header and verify its sections are hidden (collapsed) and the chevron flips.
  - [x] Click a section item and verify:
    - `select` event is emitted with the correct `LaputaSectionName`
    - the clicked item receives the active-section styling
  - [x] Remove or keep the smoke-test harness as an automated Vitest test under `agent-diva-gui/src/components/persona-memory/__tests__/SectionGroupList.spec.ts` (optional but recommended).

- [x] **Run validation gates** (AC: #8)
  - [x] `pnpm vue-tsc --noEmit` inside `agent-diva-gui` passes.
  - [x] `pnpm build` inside `agent-diva-gui` passes.
  - [x] `just fmt-check`
  - [x] `just check`
  - [x] Manual smoke test: open "Persona & Memory", confirm 14 sections are visible, groups expand/collapse, and selecting a section updates the right panel.

## Dev Notes

### Relevant architecture patterns and constraints

- **Group model is frontend-only**. Per Architecture AD-3, the Persona/Memory/Periodic/Indexes grouping must not leak into backend types or API schemas. Keep the mapping as a local constant inside `SectionGroupList.vue`.
- **Props-driven, stateless list**. `SectionGroupList.vue` does not fetch data. It receives `snapshot` and `selectedSection` from `PersonaMemoryView.vue` and emits `select` events upward. Local UI state (group expand/collapse) may live inside the component.
- **Locale-driven labels**. All user-facing strings come from `t('laputa.groups.*')`, `t('laputa.sections.*')`, and `t('laputa.status.*')`. Do not hard-code section names, group names, or status labels.
- **No rollback / diff UI**. Per PRD §4.2 and AD-5, rollback buttons and diff preview are explicitly deferred. `SectionGroupList.vue` must not introduce any rollback, diff, or history controls.
- **Theme-safe styling**. All colors, borders, radii, and shadows must use CSS variables from `agent-diva-gui/src/styles.css` so love/dark/default/miku themes work automatically.
- **No direct file writes**. This is a pure frontend component; persistence and file-system access remain in Epic 1 / `LaputaService`.

### Source tree components to touch

- `agent-diva-gui/src/components/persona-memory/SectionGroupList.vue` — new component (this story)
- `agent-diva-gui/src/components/PersonaMemoryView.vue` — integration with parent container (created in Story 2.2)
- `agent-diva-gui/src/api/desktop.ts` — import `LaputaSectionName` type (no API changes)
- `agent-diva-gui/src/locales/zh.ts` — add `laputa.groups.*`, `laputa.sections.*`, `laputa.status.*`
- `agent-diva-gui/src/locales/en.ts` — English counterparts
- `agent-diva-gui/src/styles.css` — consume existing CSS variables (no new variables required)
- Optional: `agent-diva-gui/src/components/persona-memory/__tests__/SectionGroupList.spec.ts` — smoke / unit tests

### Testing standards summary

- Use Vitest + Vue Test Utils for component tests if a persistent test file is created.
- Mock `vue-i18n` with the English locale so rendered text is stable.
- Assert that the component renders 14 section rows given a full snapshot.
- Assert that clicking a group header toggles the visibility of its children.
- Assert that clicking a section item emits `select` with the correct name and applies the active class.
- For manual smoke tests, use a sample snapshot with mixed `owned`/`tbd` statuses to verify badge styling.

### Project Structure Notes

- This story belongs to **Epic 2: 人格与记忆页面骨架与 Section 浏览** and focuses on the left-panel section list.
- It depends on `PersonaMemoryView.vue` (Story 2.2) for the page container and snapshot state.
- It precedes `SectionEditor.vue` (Story 2.4 / Story 3.1) and `HistoryModal.vue` (Story 3.4), which will consume the same `selectedSection` state.
- Keep the component focused on list rendering, grouping, selection, and visual states; do not implement editor behavior, save flow, or history modal here.

### References

- Example BMAD story structure: [Source: `_bmad-output/implementation-artifacts/stories/1-1-add-direct-edit-and-apply-to-laputa-service.md`]
- PRD FR-102 / §4: [Source: `docs/prds/prd-persona-memory-laputa-ui-2026-07-05.md`]
- Architecture spine AD-2 / AD-3 and component seed: [Source: `docs/architecture/architecture-persona-memory-laputa-ui-2026-07-05.md`]
- Visual tokens and component specs: [Source: `docs/ux/persona-memory-laputa-2026-07-05/DESIGN.md`]
- Experience flows and i18n key suggestions: [Source: `docs/ux/persona-memory-laputa-2026-07-05/EXPERIENCE.md`]
- Epic breakdown Story 2.3: [Source: `_bmad-output/planning-artifacts/epics.md`]
- Parent page container spec: [Source: `_bmad-output/implementation-artifacts/stories/2-2-create-personamemoryview-page-container.md`]
- Right-panel editor spec: [Source: `_bmad-output/implementation-artifacts/stories/2-4-view-section-content.md`]
- Laputa API types and section names: [Source: `agent-diva-gui/src/api/desktop.ts`]
- Project CSS variables: [Source: `agent-diva-gui/src/styles.css`]
- Parent mount point / sidebar routing: [Source: `agent-diva-gui/src/components/NormalMode.vue`]
- i18n structure: [Source: `agent-diva-gui/src/locales/zh.ts`, `agent-diva-gui/src/locales/en.ts`]

## Dev Agent Record

### Agent Model Used

Sisyphus-Junior (kimi-for-coding)

### Debug Log References

- Initial `v-show` visibility test used `isVisible()` which did not reflect `display: none` in happy-dom; switched to checking inline `style.display`.
- `PersonaMemoryView.vue` prop/emit binding verified as already correct from Story 2.2; no changes required.

### Completion Notes List

- [x] `SectionGroupList.vue` created with 4-group configuration, expand/collapse, active state, and status badges
- [x] `laputa.groups.*`, `laputa.sections.*`, and `laputa.status.*` keys added to both `zh.ts` and `en.ts`
- [x] `SectionGroupList.vue` integrated into `PersonaMemoryView.vue` with correct prop/emit binding
- [x] 14 sections, group toggles, and selection emit verified by smoke test
- [x] `vue-tsc --noEmit` and `pnpm build` pass
- [x] `just fmt-check && just check` clean

### File List

- `agent-diva-gui/src/components/persona-memory/SectionGroupList.vue`
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src/locales/en.ts`
- `agent-diva-gui/src/components/persona-memory/SectionGroupList.test.ts`

### Change Log

- 2026-07-05: Implemented grouped, collapsible Laputa section list with 4 groups, 14 sections, status badges, active state, and accessibility attributes. Added i18n keys for groups/sections/status. Added Vitest smoke tests. Validation passed: `vue-tsc --noEmit`, `pnpm build`, `just fmt-check`, `just check`.

(End of file)
