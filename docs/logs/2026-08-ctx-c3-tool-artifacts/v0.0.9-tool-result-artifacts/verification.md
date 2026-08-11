# CTX-C3 verification

Focused validation completed during implementation:

- `cargo test -p agent-diva-core tool_artifact --lib`
- `cargo test -p agent-diva-core security::tool_result_filter --lib`
- `cargo test -p agent-diva-tooling --lib`
- `cargo test -p agent-diva-tools --lib`
- `cargo test -p agent-diva-tools read_tool_result --lib`
- `cargo test -p agent-diva-agent tool_results --lib`
- `cargo test -p agent-diva-agent context_assembly::cache_observe --lib`
- `cargo test -p agent-diva-agent tool_assembly --lib`
- `cargo clippy -p agent-diva-core -p agent-diva-tooling -p agent-diva-tools -p agent-diva-agent --all-targets -- -D warnings`

Coverage includes restart reads, workspace/session binding, opaque-ID validation, Unicode ranges, corruption detection, item capacity rejection, session deletion, 12k inline boundary, >10 MiB fallback, read ranges, oldest-first/idempotent microcompact, current-group preservation, and expected cache deletion classification.

Final gates:

- `just fmt-check` — passed.
- `just check` — passed for the full workspace with warnings denied.
- `just test` — passed for the full workspace. The first run hit the command wrapper's 120-second timeout; the unchanged rerun with a 600-second allowance passed.
- `just ci` — passed, including full tests, manager health benchmark, sandbox feature gates, Laputa clean-break, and BML boundary checks.
- `cargo run -p agent-diva-cli -- --help` — passed; the CLI displayed its command and option surface.

The existing non-fatal `imap-proto` future-incompatibility notice and Laputa test-helper dead-code warnings remain unchanged; they do not fail the repository gates.
