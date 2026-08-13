---
baseline_commit: 63d1ea7
---

# Story 4.2: Replace Notebook Direct Solidification with Proposal Creation

Status: review

## Story

作为 Notebook 用户，
我希望报表动作创建可审查的提案，
以便 SOP、Skill、Memory 更新不能绕过 Laputa 治理。

## Acceptance Criteria

1. Given a report is open in Notebook, when I choose Create SOP Proposal, Create Skill Proposal, or Create Memory Proposal, then the UI previews target, extracted summary, evidence refs, risk, and review status.
2. Submitting creates an `EvolutionProposal`.
3. No durable authority file is modified by the Notebook action.
4. The created proposal can be opened in Evolution Inbox.

## Tasks / Subtasks

- [x] Rename visible Notebook actions and i18n copy from direct solidification to proposal creation: Create SOP Proposal, Create Skill Proposal, Create Memory Proposal. (AC: 1)
- [x] Add a proposal preview state/modal/panel showing target section, proposal type, extracted summary, evidence refs, risk level, and pending review status before submit. (AC: 1)
- [x] Add or reuse a report-to-proposal backend command that builds `EvolutionProposal` using `agent-diva-core::evolution` types and persists through Laputa `create_proposal`. (AC: 2)
- [x] Map SOP and Skill to `ProposalType::SopCreate` with target `identity`; include a sub-target marker in `proposed_patch` or metadata-compatible patch text for skill vs SOP. (AC: 2)
- [x] Map Memory update to `MemoryPatch`, `LearningNote`, `IdentityPatch`, or `RelationshipUpdate` only when the extracted content justifies that type; ambiguous extraction must return `needs_attention`-style UX instead of applying. (AC: 1, 2)
- [x] Convert old Tauri command semantics (`solidify_report_as_sop`, `solidify_report_as_skill`, `update_memory_from_report`) to proposal creation or replace their frontend usage; they must not write authority files. (AC: 2, 3)
- [x] After proposal creation, show success copy saying proposal created, and provide a link/deep-link to Evolution Inbox/proposal detail. (AC: 4)
- [x] Add tests proving Notebook actions create proposals and no authority files change until Laputa apply. (AC: 2-4)

### Review Findings

- [ ] [Review][Patch] Evolution Inbox deep-link sets `selectedProposalId` before proposals are loaded, so Notebook-created proposal links can open without loading the target detail pane [`agent-diva-gui/src/components/EvolutionView.vue:466`]
- [ ] [Review][Patch] Failed proposal preview requests leave stale preview content visible, so the modal can show one action while submitting another [`agent-diva-gui/src/components/NotebookView.vue:215`]
- [ ] [Review][Patch] Notebook proposals set `source_run_id` to the report id rather than an actual run identifier, which breaks source-run semantics and filtering [`agent-diva-gui/src-tauri/src/notebook.rs:188`]

## Dev Notes

### Architecture Context

- `docs/architecture/evo-diva-architecture-2026-06-12.md` section 7 explicitly migrates current Notebook actions from direct durable writes to proposal creation.
- Required mapping:
  - `solidify_report_as_sop` -> `SopCreate` proposal.
  - `solidify_report_as_skill` -> `SopCreate` proposal with skill sub-target.
  - `update_memory_from_report` -> `MemoryPatch`, `LearningNote`, `IdentityPatch`, or `RelationshipUpdate` proposal.
- UI may keep the same physical location in Notebook, but backend semantics must say "proposal created" until approval and apply.

### Current Code State

- `agent-diva-gui/src/components/NotebookView.vue` currently invokes `solidify_report_as_sop`, `solidify_report_as_skill`, and `update_memory_from_report`, then shows success messages implying direct writes.
- `agent-diva-gui/src/api/desktop.ts` already contains governance DTOs and helper functions for `laputa_create_proposal`, `laputa_list_proposals`, and related Laputa operations.
- `agent-diva-gui/src-tauri/src/commands.rs` already exposes `laputa_create_proposal` by proxying to manager `/laputa/proposals`.
- `agent-diva-manager/src/handlers/laputa.rs` accepts `EvolutionProposal` in `create_laputa_proposal_handler`; prefer reusing this instead of inventing a parallel proposal store.
- `agent-diva-core/src/evolution/types.rs` owns the canonical proposal envelope, proposal type routing, evidence refs, risk levels, and states.

### Implementation Guardrails

- Do not call `applyLaputaProposal` from Notebook proposal creation. Apply belongs to Evolution review flow.
- Do not write `SOUL.md`, `IDENTITY.md`, `USER.md`, `memory/MEMORY.md`, `memory/HISTORY.md`, skill files, or SOP files from Notebook actions.
- Proposal evidence must include the report as `EvidenceSource::Report`; session evidence may be attached only when supplied by Story 4.3 search.
- Keep generated `proposed_patch` reviewable and bounded. If extraction confidence is low, create a proposal with clear risk/needs-attention copy rather than silently choosing a target.
- Preserve `NotebookView.vue` markdown safety (`html: false`) and existing loading/error behavior.

### Testing Requirements

- Unit/integration tests for report-to-proposal mapping:
  - SOP report action creates `sop_create` targeting `identity`.
  - Skill report action creates `sop_create` with skill sub-target information.
  - Memory report action creates a routed proposal or returns a typed ambiguity error.
  - No authority file changes before `apply_proposal`.
- GUI/component smoke for preview -> submit -> proposal-created state.
- Minimum validation:
  - targeted Rust tests for proposal creation flow
  - `cargo test -p agent-diva-laputa`
  - GUI validation if `NotebookView.vue` or Tauri command registration changes

### References

- `_bmad-output/planning-artifacts/epics.md` - Epic 4 Story 4.2.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` - section 7 Notebook migration.
- `_bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md` - GOV-FR101, GOV-FR201, GOV-FR501-GOV-FR503.
- `docs/prd-report-system/prd.md` - FR-6, FR-7, FR-8.
- `agent-diva-gui/src/components/NotebookView.vue` - current direct-action UI.
- `agent-diva-gui/src/api/desktop.ts` - existing Laputa/Evolution frontend DTOs.
- `agent-diva-gui/src-tauri/src/commands.rs` - existing Tauri Laputa proxy commands.
- `agent-diva-core/src/evolution/types.rs` - canonical proposal types and routing.

## Previous Story Intelligence

- Story 4.1 should establish authoritative report loading paths and report DTOs. Reuse those report IDs/content/evidence fields instead of reparsing files in the frontend.
- Epic 2 already created Evolution Inbox/detail/action surfaces; use deep-link/navigation into that surface rather than creating a second review UI in Notebook.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-15: Story context prepared from Epic 4, EVO-DIVA architecture section 7, current Notebook commands, and existing Laputa Tauri/manager proposal APIs.
- 2026-06-16: Implemented Notebook report-to-proposal preview/create flow and validated with `cargo test -p agent-diva-gui notebook::tests::`, `cargo test -p agent-diva-laputa`, `pnpm --dir agent-diva-gui build`, `just fmt-check`, `just check`, and `just test`.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.
- Replaced visible Notebook direct-write actions with Create SOP Proposal, Create Skill Proposal, and Create Memory Proposal.
- Added preview and submit state in Notebook with target section, proposal type, extracted summary, report evidence refs, risk, and review status.
- Added Tauri report-to-proposal preview/create commands that build `EvolutionProposal` values from `agent-diva-core::evolution` types and persist through the existing Laputa proposal endpoint.
- Added mapping coverage for SOP/Skill `SopCreate` proposals targeting `identity`, routed memory proposals, ambiguous memory `needs_attention`, and no authority-file mutation before apply.
- Added Evolution Inbox deep-link after proposal creation.

### File List

- `_bmad-output/implementation-artifacts/4-2-replace-notebook-direct-solidification-with-proposal-creation.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva-gui/src-tauri/src/lib.rs`
- `agent-diva-gui/src-tauri/src/notebook.rs`
- `agent-diva-gui/src/components/NormalMode.vue`
- `agent-diva-gui/src/components/NotebookView.vue`
- `agent-diva-gui/src/locales/en.ts`
- `agent-diva-gui/src/locales/zh.ts`
- `docs/logs/2026-06-notebook-proposal-creation/v0.0.1-notebook-proposal-creation/summary.md`
- `docs/logs/2026-06-notebook-proposal-creation/v0.0.1-notebook-proposal-creation/verification.md`
- `docs/logs/2026-06-notebook-proposal-creation/v0.0.1-notebook-proposal-creation/release.md`
- `docs/logs/2026-06-notebook-proposal-creation/v0.0.1-notebook-proposal-creation/acceptance.md`

### Change Log

- 2026-06-15: Created ready-for-dev story for Notebook proposal creation migration.
- 2026-06-16: Implemented Notebook proposal creation migration and moved story to review.
