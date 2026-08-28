# agent-diva-migration

## OVERVIEW

Offline migration utility (`agent-diva-migrate` binary). Imports legacy Memory authority into Embedded Laputa, backfills AutoDream evidence, and normalizes workspace identity.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| CLI entry and command enum | `src/main.rs` |
| Memory import / DryRun / Rollback | `src/typed_memory.rs` |
| Experience evidence backfill | `src/experience.rs` |
| Workspace identity migration | `src/workspace_identity.rs` |

## CONVENTIONS

- `Apply` operations require explicit `--features memory|experience|workspace_identity`.
- `DryRun` and `Rollback` are always allowed.
- Always create a verified backup before `Apply`; restore it on `Rollback`.

## ANTI-PATTERNS

- Do not run `Apply` against a production workspace without a verified backup.
- Do not bypass the `--features` gate for destructive operations.
- Do not treat this crate as a runtime dependency; it is an offline utility.

## NOTES

- Run via `just migrate -- <args>` or `cargo run -p agent-diva-migration -- <args>`.
