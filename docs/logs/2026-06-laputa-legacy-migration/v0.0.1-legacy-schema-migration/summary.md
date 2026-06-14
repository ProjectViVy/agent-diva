# Story 6.1 Legacy Schema Migration Summary

Date: 2026-06-14

## Changed

- Added explicit `LaputaMigration` API for legacy authority migration.
- Added `.laputa/staging/` and timestamped `.laputa/legacy/{timestamp}/` path helpers.
- Migrates legacy templates into Laputa sections without deleting source files.
- Backs up legacy sources, including `BOOTSTRAP.md`, while keeping bootstrap out of persistent runtime authority sections.
- Preserves unsupported or future-owned material as explicit `status=tbd` payloads.
- Updates `.laputa/state.json` to schema version `1.1.0` only after staged section writes succeed.
- Cleaned two existing `agent-diva-laputa` clippy warnings so the crate validation gate passes.

## Impact

Scope is limited to `agent-diva-laputa` storage and migration behavior. No automatic migration is triggered by `LaputaStorage::open`; callers must invoke the explicit migration API.
