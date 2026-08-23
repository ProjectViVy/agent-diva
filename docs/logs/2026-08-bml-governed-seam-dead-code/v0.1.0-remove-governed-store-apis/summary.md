# Remove unused BML governed store APIs

- Date: 2026-08-23
- Slice: `BML-GOVERNED-SEAM-DEAD-CODE`
- Branch / worktree: `chore/bml-governed-seam-dead-code` /
  `C:\Users\Administrator\Desktop\morediva\agent-diva-bml-governed-seam`
- Base: local `dev` @ `1471f1e6`

## Goal

Delete the unused `TypedMemoryStore::put_governed` / `rollback_governed`
code path left after S5 removed the governance coordinator. Keep the BML
table structure (D4 §3.2).

## Changes

- Removed `GovernedMemoryApply`, `put_governed`, `rollback_governed`,
  and `TypedMemoryStoreError::{InvalidReceipt, ApplyIdempotencyConflict}`.
- `put_inner` no longer takes a governed argument and no longer reads or
  writes `memory_apply_journal`.
- `CREATE TABLE IF NOT EXISTS memory_apply_journal` and `SCHEMA_VERSION = 1`
  are unchanged.
- `bml/mod.rs` and root `AGENTS.md` now describe the deleted seam;
  `bml_boundary_guard` still scans `.put_governed(` / `.rollback_governed(`
  so the names cannot return.

## Out of scope

- No DROP/ALTER of BML tables.
- `MEMORY-CRUD-PROPOSAL-CREATED-DEAD-ENUM` left open.
- `LAPUTA-TESTS-1.94-ALL-TARGETS-CLIPPY` left open.
- Historical research snapshots under `docs/research/` were not rewritten.
