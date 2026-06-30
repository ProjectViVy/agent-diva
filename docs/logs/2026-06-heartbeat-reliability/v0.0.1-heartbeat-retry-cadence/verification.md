# Verification

- `cargo fmt --all`
- `cargo test -p agent-diva-core heartbeat -- --nocapture`
- `cargo test -p agent-diva-core config::reload_plan -- --nocapture`
- `cargo test -p agent-diva-core config::hot_reload -- --nocapture`
- `cargo test -p agent-diva-core config::migrate -- --nocapture`
- `cargo test -p agent-diva-cli --test config_commands -- --nocapture`
- `cargo check -p agent-diva-core -p agent-diva-manager`
- `just fmt-check`

Known unrelated workspace blockers:

- `just check` fails on existing clippy-denied issues in `agent-diva-tooling` and `agent-diva-providers`.
- `just test` fails on an existing `agent-diva-migration` assertion that still references removed `providers.openai` schema.
