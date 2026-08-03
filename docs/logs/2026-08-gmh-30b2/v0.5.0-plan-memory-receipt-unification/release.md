# GMH-30B2 Release

## Delivery

This is a source-level, additive SQLite schema update delivered as one focused
commit. No push, deployment, desktop launch, provider request, secret access,
profile mutation, administrator action, network policy change, or system policy
change was performed.

The new Plan domain mapping table is created idempotently. Existing Memory
journal states remain readable, including the legacy authority-committed
recovery state.

## Rollback

Revert the focused GMH-30B2 commit. The added domain table is harmless if left
in place. Governance ledger events are append-only facts and are not deleted by
rollback; terminal or unconsumed states continue to fail closed.

## Next stage

Proceed to GMH-31 for Manager API/SSE/Tauri contract work. Do not perform the
final human smoke until all M3 implementation stages are complete.
