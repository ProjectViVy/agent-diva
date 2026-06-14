# Story 3.3 Acceptance

## Acceptance Checks

- Trigger a manual AutoDream run with seeded session and Laputa evidence.
- Execute `AutoDreamService::execute_reflection_worker(run_id)`.
- Verify stage records are produced in this exact order: Orient, Gather, Consolidate, Propose.
- Verify restricted profile denies arbitrary shell, Mentle writes, direct Laputa authority writes, and monthly report writes.
- Verify success writes a completed run, appends worker events, creates a pending-review proposal through Laputa API, removes the lock, and updates checkpoint.
- Verify timeout, cancellation, and failure produce diagnostics, terminal run state, events, lock cleanup, and no checkpoint success update.

## Automated Coverage

- Covered by `agent-diva-autodream/tests/worker.rs`.
