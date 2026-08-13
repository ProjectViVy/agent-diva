# Acceptance

1. From the repository root, `cargo test -p agent-diva-e2e -- --nocapture` starts successfully instead of failing with a workspace-membership error.
2. From the repository root, `just e2e-test` is available and targets the dedicated real-provider E2E lane.
3. Without E2E credentials, the real-provider scenario tests skip cleanly rather than failing due to Cargo/workspace configuration.
4. Existing default workspace validation semantics stay unchanged: `just test` still means normal workspace tests, not opt-in live-provider E2E.
