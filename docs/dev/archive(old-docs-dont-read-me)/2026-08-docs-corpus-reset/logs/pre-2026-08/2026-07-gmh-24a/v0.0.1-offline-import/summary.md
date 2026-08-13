# GMH-24A Offline Import Summary

The migration CLI now exposes explicit `memory dry-run`, `memory apply`, and
`memory rollback` operations for Embedded Laputa. Inputs are limited to
explicit legacy owner Markdown and supported Laputa section JSON beneath a
declared source root. Mentle paths and symlink escapes are rejected.

Dry-run is filesystem-side-effect free for a new workspace. Apply writes
deterministic normalized records in one SQLite transaction after a verified
backup. A migration manifest supports idempotent replay and restoration.

Configuration now owns a strict `memory.authority_mode` with `legacy`,
`shadow`, and `typed` values; missing configuration remains `legacy`.
