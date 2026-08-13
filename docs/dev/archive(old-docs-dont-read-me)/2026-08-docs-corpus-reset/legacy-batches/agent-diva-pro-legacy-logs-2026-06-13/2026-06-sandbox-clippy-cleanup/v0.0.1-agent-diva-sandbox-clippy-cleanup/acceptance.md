# Acceptance

1. Run `cargo clippy -p agent-diva-sandbox --all-targets -- -D warnings`.
2. Confirm the command exits with code `0`.
3. Run `cargo test -p agent-diva-sandbox`.
4. Confirm all crate tests pass.
5. Confirm the only code changes are mechanical clippy cleanups in:
   - `agent-diva-sandbox/src/platform/windows.rs`
   - `agent-diva-sandbox/src/orchestrator.rs`
