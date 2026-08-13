---
baseline_commit: 5d072f6
---

# Story 3.3: Execute Restricted Reflection Prompt

Status: review

## Story

As the system,
I want AutoDream to run a restricted reflection worker,
so that proposal generation cannot directly mutate durable authority.

## Acceptance Criteria

1. Given inputs have been collected, when the worker runs, then it executes the Orient, Gather, Consolidate, and Propose stages.
2. The worker uses a restricted tool profile with no arbitrary shell, no Mentle writes, and no direct authority-write tools.
3. Any proposal creation uses the Laputa proposal API rather than direct `.laputa` writes.
4. Timeout, cancellation, success, and failure outcomes are recorded.
5. Failures produce user-visible diagnostics and do not update success checkpoint.

## Tasks / Subtasks

- [x] Add `agent-diva-autodream/src/worker.rs` or `distiller.rs` for the four-stage reflection worker. (AC: 1)
- [x] Define a restricted execution profile that allows bounded session/Laputa reads, AutoDream output writes, and Laputa proposal API calls only. (AC: 2, 3)
- [x] Implement Orient, Gather, Consolidate, and Propose stage orchestration with structured stage status. (AC: 1, 4)
- [x] Wire cancellation and timeout checks into each stage. (AC: 4)
- [x] Record success, failure, cancellation, and timeout outcomes in run records and `events.jsonl`. (AC: 4, 5)
- [x] Ensure failures leave checkpoint unchanged and produce diagnostics consumable by Evolution Runs/Settings. (AC: 5)
- [x] Add tests for stage order, restricted profile denial, timeout, cancellation, failure diagnostics, and checkpoint non-update on failure. (AC: 1-5)

## Dev Notes

### Architecture Context

- AutoDream prompt execution must be restricted per `docs/architecture/evo-diva-architecture-2026-06-12.md` §6.4: read sessions, read Laputa through API, write `.agent-diva/autodream/*`, create proposals through Laputa API, no arbitrary shell, no Mentle writes, no direct `.laputa` writes, no monthly report writes.
- The older `docs/architecture/autodream-architecture-2026-06-12.md` describes forked subagent/thread execution. In EVO-DIVA v1, keep this worker thin and bounded; do not turn it into a scheduler or agent runtime.
- Proposal application is out of scope. The worker can create review-required proposals only.

### Current Code State

- `agent-diva-agent` owns the main agent loop and subagent concepts; AutoDream must not entangle the live conversation path.
- `agent-diva-tools` includes powerful tools such as shell/filesystem. Do not hand the full registry to AutoDream.
- Laputa proposal creation already exists from Epic 1 and should be the only proposal persistence path.

### Implementation Guardrails

- Do not add arbitrary shell access to AutoDream.
- Do not update success checkpoint until the whole run reaches success.
- Do not swallow partial-stage failures; write them into run diagnostics.
- Prefer deterministic worker tests with fake input and fake proposal sink before wiring real providers.
- If real LLM invocation is not available in this story, implement the worker boundary and a deterministic test worker, then leave provider execution behind a trait for a later enhancement only if the AC remains satisfied.

### Testing Requirements

- Minimum targeted validation: `cargo test -p agent-diva-autodream worker`.
- Add a test proving direct `.laputa` write tools are absent from the restricted profile.
- Add a test proving cancellation changes run state and does not mark the run completed.

## Previous Story Intelligence

- Story 3.1 owns lifecycle, lock, run state, and cancellation.
- Story 3.2 owns bounded input collection. This story should consume its input bundle rather than reaching back into session/Laputa sources directly.

## Completion Note

Ultimate context engine analysis completed - comprehensive developer guide created.

## Dev Agent Record

### Agent Model Used

GPT-5 Codex

### Debug Log References

- 2026-06-14: Loaded `bmad-dev-story` workflow, project context, sprint status, Story 3.3 requirements, and prior AutoDream Story 3.1/3.2 implementation records.
- 2026-06-14: Added restricted AutoDream reflection worker with Orient, Gather, Consolidate, and Propose stages; success/failure/cancellation/timeout terminal handling; run record, event, and checkpoint semantics.
- 2026-06-14: Added worker tests for stage order, restricted profile denial, timeout, cancellation, failure diagnostics, checkpoint non-update, and Laputa API proposal boundary.
- 2026-06-14: Targeted AutoDream validation passed. Workspace `just check` and `just test` remain blocked by unrelated pre-existing `agent-diva-sandbox` compile issues and `agent-diva-laputa` clippy issues recorded in `TODOLIST.md`.

### Completion Notes List

- Added `agent-diva-autodream/src/worker.rs` with `AutoDreamWorker`, `AutoDreamRestrictedProfile`, structured stage records, outcome reporting, timeout checks, cancellation checks, and deterministic proposal generation through `AutoDreamOutputEmitter`.
- Exposed `AutoDreamService::execute_reflection_worker` as the service-level entry point while preserving AutoDream's restricted boundary and avoiding live agent-loop/subagent/tool-registry entanglement.
- Worker success now marks run completed, records events, removes the active lock, and updates checkpoint only after all four stages complete.
- Worker failure, cancellation, and timeout now record diagnostics in the run record and `events.jsonl`, remove the active lock, and leave the success checkpoint unchanged.
- Added `agent-diva-autodream/tests/worker.rs` covering the required stage, restriction, outcome, diagnostic, checkpoint, and no-direct-authority-write scenarios.

### File List

- `TODOLIST.md`
- `_bmad-output/implementation-artifacts/3-3-execute-restricted-reflection-prompt.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`
- `agent-diva-autodream/src/lib.rs`
- `agent-diva-autodream/src/service.rs`
- `agent-diva-autodream/src/worker.rs`
- `agent-diva-autodream/tests/worker.rs`
- `docs/logs/2026-06-autodream-restricted-worker/v0.0.1-restricted-reflection-worker/acceptance.md`
- `docs/logs/2026-06-autodream-restricted-worker/v0.0.1-restricted-reflection-worker/release.md`
- `docs/logs/2026-06-autodream-restricted-worker/v0.0.1-restricted-reflection-worker/summary.md`
- `docs/logs/2026-06-autodream-restricted-worker/v0.0.1-restricted-reflection-worker/verification.md`

### Change Log

- 2026-06-14: Implemented restricted AutoDream reflection worker and moved story to review.
