---
baseline_commit: 8a1114d
---

# Story 5.1: Plug Applied Laputa Reads into MemoryProvider

Status: review

## Story

As Diva runtime,
I want to consume applied authority through `MemoryProvider`,
so that prompts use reviewed state without depending on Laputa internals.

## Acceptance Criteria

1. Given Laputa proposal, apply, changelog, and read APIs exist, when runtime context is built, then `MemoryProvider` can read applied Laputa snapshot or section data.
2. Unapplied proposals are excluded from default prompt context.
3. After Laputa is available, `ContextBuilder`, subagent identity assembly, and runtime prompt assembly do not directly read `SOUL.md`, `IDENTITY.md`, `USER.md`, `memory/MEMORY.md`, or `memory/HISTORY.md` as authority.
4. Legacy authority files can be consumed only through migration or a compatibility adapter, with legacy source marked explicitly and unapplied content excluded from default prompts.
5. Read failures degrade safely without creating authority writes.
6. Tests cover prompt consumption of applied and unapplied changes.

## Tasks / Subtasks

- [x] Add a read-only Laputa-backed `MemoryProvider` adapter, keeping storage-specific types outside runtime prompt assembly. (AC: 1, 5)
- [x] Wire `ContextBuilder` and `AgentLoop` construction to prefer the Laputa adapter when available. (AC: 1, 3)
- [x] Render applied authority sections through explicit prompt blocks that distinguish authority content from untrusted evidence. (AC: 1, 2)
- [x] Remove default authority reads from legacy `SOUL.md`, `IDENTITY.md`, `USER.md`, `memory/MEMORY.md`, and `memory/HISTORY.md` in runtime prompt paths. (AC: 3)
- [x] Add a compatibility/migration read path for legacy files that marks legacy provenance and remains excluded from default authority prompt context unless migrated/applied. (AC: 4)
- [x] Ensure Laputa read errors produce degraded prompts and diagnostics without writes, proposal creation, or fallback authority mutation. (AC: 5)
- [x] Add tests for applied snapshot rendering, unapplied proposal exclusion, legacy-file non-authority behavior, and read-failure degradation. (AC: 1-6)

## Dev Notes

### Architecture Context

- `docs/architecture/evo-diva-architecture-2026-06-12.md` section 5.7 defines `agent-diva-core::memory::MemoryProvider` as the prompt/runtime adapter boundary.
- The required implementation order has now reached this story's precondition: file store, proposal API, apply/changelog/audit/rollback, and read-only snapshot/section APIs exist from Epic 1.
- Phase 5 of the architecture says to add the Laputa read adapter for `MemoryProvider`, render authority sections using Always/Relevant/Archive rules, and distinguish authority content from untrusted evidence.

### Current Code State

- `agent-diva-agent/src/context.rs` currently constructs `MemoryManager` as the default `MemoryProvider`.
- `ContextBuilder` still contains direct legacy prompt instructions and reads for `SOUL.md`, `IDENTITY.md`, `USER.md`, and `memory/MEMORY.md`/`memory/HISTORY.md` semantics.
- `agent-diva-agent/src/agent_loop.rs` injects a `MemoryProvider` during agent construction and has existing tests around prefetch/sync failure behavior.
- `agent-diva-laputa` already exposes snapshot and section reads through the service/manager/Tauri boundary from Story 1.5.

### Implementation Guardrails

- Do not read `.laputa/state.json` directly outside `agent-diva-laputa`.
- Do not call proposal apply or any write path from prompt rendering or `MemoryProvider::sync_turn`.
- Do not use unapplied proposals as default authority. If they are displayed, label them as pending/untrusted evidence.
- Preserve existing live conversation behavior when Laputa is unavailable; failure must degrade context, not block chat.
- Keep legacy authority file access explicit and temporary. It is migration/compatibility input, not default authority.

### Testing Requirements

- Unit tests in `agent-diva-agent/src/context.rs` or focused integration tests should assert:
  - applied Laputa sections appear in prompt payload;
  - unapplied proposals do not appear as authority;
  - legacy files are not directly rendered as authority by default;
  - Laputa read failure does not write authority and still builds a usable prompt.
- Run targeted validation:
  - `cargo test -p agent-diva-agent context`
  - `cargo test -p agent-diva-laputa`
  - `cargo check -p agent-diva-agent`
- If runtime construction changes manager or GUI command boundaries, add the relevant crate checks.

### References

- `_bmad-output/planning-artifacts/epics.md` - Epic 5 and Story 5.1 requirements.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` - MemoryProvider boundary and Phase 5 prompt consumption plan.
- `_bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md` - FR-2xx authority contract and prompt rendering NFR.
- `docs/prds/prd-laputa-2026-06-12/prd.md` - Laputa read APIs and authority file migration constraints.
- `agent-diva-agent/src/context.rs` - current prompt assembly and legacy authority reads.
- `agent-diva-agent/src/agent_loop.rs` - runtime `MemoryProvider` wiring.

## Previous Story Intelligence

- Story 1.5 completed the Laputa read/changelog/rollback/event API surface. Runtime must call that API surface instead of physical state files.
- Story 1.6 made session saves atomic. This story should preserve session compaction behavior while changing durable authority consumption.

## Dev Agent Record

### Agent Model Used

Codex GPT-5.

### Debug Log References

- 2026-06-14: Story context prepared from Epic 5, EVO-DIVA architecture sections 5.7 and 12 Phase 5, Governance PRD FR-2xx, Laputa PRD read boundaries, and existing `ContextBuilder`/`AgentLoop` prompt wiring.
- 2026-06-14: Started implementation for story 5.1; preserving existing `baseline_commit: 8a1114d` per workflow rule.
- 2026-06-15: Implemented read-only `LaputaMemoryProvider`, wired default agent/manager runtime selection, removed legacy authority prompt reads, and validated targeted agent/laputa tests.

### Completion Notes List

- Added `agent_diva_laputa::LaputaMemoryProvider`, a read-only `MemoryProvider` adapter that renders applied Laputa authority sections and degrades safely on empty/read-failure states.
- Wired `ContextBuilder`, `AgentLoop`, `with_toolset`, and manager runtime construction to prefer the Laputa adapter when `.laputa` exists, with fallback to `MemoryManager`.
- Removed default prompt authority reads from legacy `SOUL.md`, `IDENTITY.md`, `USER.md`, `memory/MEMORY.md`, and `memory/HISTORY.md`; legacy files remain migration/compatibility inputs through Laputa migration.
- Updated subagent prompt assembly to use applied Laputa authority context instead of inherited legacy identity files.
- Added/updated tests for applied section rendering, pending proposal exclusion, legacy non-authority behavior, and read-failure degradation.
- Validation note: `cargo check -p agent-diva-manager` remains blocked by a pre-existing unrelated AutoDream error-match exhaustiveness issue captured in `TODOLIST.md`.

### File List

- `_bmad-output/implementation-artifacts/5-1-plug-applied-laputa-reads-into-memoryprovider.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `Cargo.lock`
- `TODOLIST.md`
- `agent-diva-agent/Cargo.toml`
- `agent-diva-agent/src/agent_loop.rs`
- `agent-diva-agent/src/context.rs`
- `agent-diva-agent/src/subagent.rs`
- `agent-diva-laputa/Cargo.toml`
- `agent-diva-laputa/src/lib.rs`
- `agent-diva-laputa/src/memory_provider.rs`
- `agent-diva-manager/src/runtime.rs`
- `docs/logs/2026-06-laputa-memory-provider/v0.0.1-applied-laputa-memory-provider/acceptance.md`
- `docs/logs/2026-06-laputa-memory-provider/v0.0.1-applied-laputa-memory-provider/release.md`
- `docs/logs/2026-06-laputa-memory-provider/v0.0.1-applied-laputa-memory-provider/summary.md`
- `docs/logs/2026-06-laputa-memory-provider/v0.0.1-applied-laputa-memory-provider/verification.md`

### Change Log

- 2026-06-14: Created ready-for-dev story for applied Laputa prompt consumption through `MemoryProvider`.
- 2026-06-15: Implemented applied Laputa `MemoryProvider` runtime consumption and moved story to review.
