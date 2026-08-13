# Verification

## Targeted Tests

- `cargo test -p agent-diva-core security_integration -- --nocapture`
- `cargo test -p agent-diva-manager skill_service -- --nocapture`
- `cargo test -p agent-diva-tooling registry --lib`
- `cargo test -p agent-diva-core supervised:: -- --nocapture`
- `cargo test -p agent-diva-agent test_process_direct_blocks_injection_before_provider_call --lib`
- `cargo test -p agent-diva-agent test_process_direct_sanitizes_pii_before_provider_call --lib`
- `cargo test -p agent-diva-agent test_process_direct_rejects_when_session_budget_exceeded --lib`
- `cargo test -p agent-diva-agent test_process_direct_appends_usage_to_token_ledger --lib`

## Result

- All commands above passed.
- `agent-diva-e2e` targeted tests were validated by the child-agent change set, but not rerun from the root workspace because that crate is not a root workspace member.
