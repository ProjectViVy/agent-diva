# GMH-23B Verification

- `cargo test -p agent-diva-laputa --test typed_store`: 8 passed; one explicit performance test is ignored by the default suite.
- Explicit 10,000-record debug performance gate: passed in 51.39s; top-8 FTS search P95 was **62.8675ms** on Windows, below the 200ms gate.
- `cargo test -p agent-diva-laputa`: passed all Laputa unit, integration, and doc tests.
- `cargo clippy -p agent-diva-laputa --lib --test typed_store -- -D warnings`: passed.
- `just fmt-check`: passed.
- `just check`: passed.
- `just test`: passed for the complete workspace.
- Dependency inspection: the crate reuses `sqlx 0.7.4`; no `rusqlite`, new Mentle, or LLVM dependency was introduced.
- Windows FTS5 was exercised by creating, inserting, reopening, and querying the real virtual table under a path containing spaces.

Deferred workspace facts are recorded in `TODOLIST.md`: Rust 1.80 Cargo cannot
parse the legacy Mentle `time-core` Edition-2024 chain, and current stable
clippy flags four unrelated pre-existing service-test assertions.
