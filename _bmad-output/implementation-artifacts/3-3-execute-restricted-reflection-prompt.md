---
baseline_commit: 5d072f6
---

# Story 3.3: Execute Restricted Reflection Prompt

Status: ready-for-dev

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

- [ ] Add `agent-diva-autodream/src/worker.rs` or `distiller.rs` for the four-stage reflection worker. (AC: 1)
- [ ] Define a restricted execution profile that allows bounded session/Laputa reads, AutoDream output writes, and Laputa proposal API calls only. (AC: 2, 3)
- [ ] Implement Orient, Gather, Consolidate, and Propose stage orchestration with structured stage status. (AC: 1, 4)
- [ ] Wire cancellation and timeout checks into each stage. (AC: 4)
- [ ] Record success, failure, cancellation, and timeout outcomes in run records and `events.jsonl`. (AC: 4, 5)
- [ ] Ensure failures leave checkpoint unchanged and produce diagnostics consumable by Evolution Runs/Settings. (AC: 5)
- [ ] Add tests for stage order, restricted profile denial, timeout, cancellation, failure diagnostics, and checkpoint non-update on failure. (AC: 1-5)

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
