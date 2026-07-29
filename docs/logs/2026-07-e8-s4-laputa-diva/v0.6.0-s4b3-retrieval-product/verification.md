# S4b-3 Verification

- Windows FTS5 store suite: passed, including scope, tombstone, `RemoteOnly`, restart, schema rejection, and 10,000-record P95 under 200 ms.
- Manager application routing and Manager-local HTTP suites: passed.
- GUI lint/build and 507-test suite: passed.
- `cargo tree -i rusqlite --depth 1`: one `rusqlite 0.31.0`.
- GUI `invoke(` leakage outside `src/api/**`: zero.
- `just ci`: passed before commit.

`cargo +1.80.0 check` is not green: Cargo 1.80 cannot parse the existing edition-2024 `hashbrown 0.17.1` manifest selected by Tauri 2.11.5's transitive dependency graph. No dependency was silently downgraded and workspace MSRV was not changed; follow-up is tracked as `RG-E8-S4b-MS1`.

No manual GUI testing was performed.
