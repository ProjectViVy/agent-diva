# Story 1.4 Acceptance

## Acceptance Steps

1. Create a pending proposal targeting a supported Laputa section, approve it, then call `apply_proposal`.
2. Confirm Laputa writes the target section through `.laputa/sections`, stages rollback data, appends changelog, appends audit, and marks the proposal `applied`.
3. Attempt apply for non-approved proposal, unauthorized target, schema mismatch, and unresolved conflict cases; confirm typed errors and no authority mutation.
4. Inject a mid-apply failure after section write; confirm the previous section content is restored and the proposal is marked `needs_attention`.
5. Hold the apply lock and call apply; confirm lock timeout is returned.

## Result

Acceptance is covered by `agent-diva-laputa/tests/apply.rs`.
