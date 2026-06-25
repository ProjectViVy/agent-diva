# Verification

## Commands

- `cargo fmt --all`
- `cargo test -p agent-diva-core audit -- --nocapture`
- `cargo test -p agent-diva-gui audit_reader -- --nocapture`
- `cargo test -p agent-diva-agent test_process_inbound_emits_audit_bus_events -- --nocapture`
- `just fmt-check`
- `just check`
- `cargo test`
- `npm ci`
- `npm run build`

## Results

- `just fmt-check`: passed.
- `just check`: passed after adding a targeted `clippy::module_inception` allow on `agent-diva-core/src/audit/mod.rs` to preserve the requested `src/audit/audit.rs` structure.
- `cargo test`: passed across the workspace.
- `npm run build`: passed after installing missing frontend dependencies with `npm ci`.

## Notes

- `npm ci` reported 10 known frontend dependency vulnerabilities (6 moderate, 4 high). This was logged to `TODOLIST.md` as follow-up `H-7`.
- `cargo test` emitted pre-existing warnings in unrelated test code and a future-incompatibility note for `imap-proto v0.10.2`, but the test run completed successfully.
