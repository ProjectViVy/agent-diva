# Verification

- `cargo test -p agent-diva-state --test laputa_store`: 3 passed.
- `cargo test -p agent-diva-laputa`: passed.
- `cargo test -p agent-diva-cli --test laputa_smoke`: passed.
- Windows bundled SQLite FTS5 creation/query: passed.
- Unknown schema, stale CAS, restart recovery, and profile isolation: passed.
- `cargo tree -i rusqlite --depth 1`: one rusqlite 0.31 runtime version.
