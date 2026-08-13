# Acceptance

## User/Product Steps

1. Load the workspace manifest and confirm `agent-diva-laputa` is a member.
2. Run `cargo test -p agent-diva-laputa`.
3. Confirm `LaputaStorage::open(root)` creates an idempotent `.laputa/` layout with `state.json`, proposals, changelog, audit, rollback, migrations, locks, legacy staging, and section paths.
4. Confirm writes go through `atomic_write` or `atomic_write_json`.
5. Confirm lock acquisition times out for fresh held locks and recovers stale lock files.

## Result

Acceptance checks are covered by `agent-diva-laputa/tests/storage.rs`.
