# Verification

- `cargo test -p agent-diva-manager prepared_journal_recovers_commit_then_consumes_receipt --lib -- --nocapture`
- `cargo fmt --all -- --check`

Both commands passed on 2026-08-03. The focused Manager test ran one matching
recovery scenario and workspace formatting remained unchanged.
