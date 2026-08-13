# Verification

- Passed: `cargo check -p agent-diva-agent -p agent-diva-manager -p agent-diva-gui`.
- Passed: focused agent phase resolver and full assembly phase matrix tests.
- Passed: `cargo test -p agent-diva-core planning::policy --lib` (5 tests).
- `just fmt-check && just check && just test` exceeded the local 64-second command cap without reporting a failure; it must be rerun in an environment without that cap before release.
