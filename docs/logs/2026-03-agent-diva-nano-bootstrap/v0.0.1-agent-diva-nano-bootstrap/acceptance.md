# Acceptance

1. Build the default CLI:
   - `cargo check -p agent-diva-cli --features full`
2. Build the nano CLI path:
   - `cargo check -p agent-diva-cli --no-default-features --features nano`
3. Confirm the workspace now contains `agent-diva-nano`.
4. Confirm `agent-diva-cli/Cargo.toml` defines `full` and `nano` features.
5. Confirm `gateway run` is routed through `agent-diva-nano::run_local_gateway`.
6. Confirm `agent-diva-manager` source files remain present and were not deleted.
