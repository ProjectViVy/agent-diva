# GMH-23 Stage 1 Acceptance

## Product acceptance

1. Open a temporary Laputa workspace with applied `memory_md` and `history_md`.
2. Call `MemoryProvider::sync_turn` with a memory update and history entry.
3. Observe two proposals in the proposal repository:
   `memory_patch` at medium risk and `history_patch` at low risk.
4. Confirm both proposals are `pending_review` and carry session evidence.
5. Confirm applied authority files retain their original content.
6. Call `sync_turn` with blank inputs and confirm no proposal is created.

## Deferred acceptance

This stage does not claim completion of GMH-23. GUI/import proposalization,
configurable low-risk auto-apply, high-risk human approval, edit-and-approve,
conflict handling, compensation, and forgetting remain later stages recorded in
`TODOLIST.md`.
