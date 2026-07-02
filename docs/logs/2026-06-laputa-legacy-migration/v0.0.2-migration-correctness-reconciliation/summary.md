# Laputa Migration Correctness Reconciliation

Date: 2026-06-17

## Changed

- Added the Laputa write lock around legacy migration commits.
- Merged migrated `schema_version` and `legacy_migration` data into existing `state.json` content.
- Expanded legacy discovery to include root-level `MEMORY.md` and `HISTORY.md`.
- Removed `BOOTSTRAP.md` from runtime prompt injection.
- Made Laputa read APIs report the schema version stored in `.laputa/state.json`.

## Impact

This closes the remaining Story 6.1 migration correctness gaps without changing the public migration entrypoint shape.
