# GMH-23C Shadow Recall Verification

Focused results:

- Core Recall tests: 11 passed.
- Laputa Recall integration tests: 5 passed; the explicit performance fixture
  is ignored during the ordinary suite.
- Typed-store regression tests: 8 passed; the GMH-23B performance fixture
  remains explicitly ignored during the ordinary suite.
- G2C labelled fixture: recall@8 100%; restricted injection rate 0; duplicate
  rate 0.
- Windows debug performance fixture: 10,000 records, top-8, 20 samples,
  P95 71.1659 ms against the `<200 ms` gate.
- Focused `cargo clippy -D warnings` passed for Core library, Laputa library,
  the Recall integration target, and the typed-store integration target.

The broader all-test-target clippy command continues to report unrelated
pre-existing Core test warnings and the four previously recorded
`agent-diva-laputa/tests/service.rs` `int_plus_one` findings. GMH-23C does not
modify those files.

`cargo +1.80.0 check -p agent-diva-laputa` remains blocked before compilation
because the existing lock selects `base64ct 1.8.3`, whose Edition-2024 manifest
cannot be parsed by Cargo 1.80. This is the already-recorded legacy dependency
chain problem and GMH-23C does not change the lockfile.

The Laputa dependency tree contains the existing `sqlx 0.7.4` SQLite runtime
and contains no `rusqlite`, `memtle`, or LLVM package.

Workspace gates are recorded after their final execution below:

- `just fmt-check`: passed.
- `just check`: passed for the complete default workspace.
- `just test`: final complete workspace rerun passed in 196.5 seconds.

One intervening run after the Rust 1.80 probe failed with widespread Tauri
macro/type inference errors. An isolated default-toolchain GUI rebuild passed,
then the complete workspace rerun passed. Dedicated MSRV target-directory
isolation is recorded in `TODOLIST.md`.
