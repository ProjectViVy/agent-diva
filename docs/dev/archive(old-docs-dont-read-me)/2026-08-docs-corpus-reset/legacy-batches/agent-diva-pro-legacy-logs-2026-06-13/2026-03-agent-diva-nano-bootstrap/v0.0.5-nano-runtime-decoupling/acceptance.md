# Acceptance

1. Confirm `agent-diva-nano/src/runtime.rs` defines its own `run_local_gateway` and `GatewayRuntimeConfig` instead of re-exporting them from manager.
2. Confirm `agent-diva-nano/Cargo.toml` no longer declares `agent-diva-manager` as a dependency.
3. Confirm `cargo check -p agent-diva-nano` and `cargo test -p agent-diva-nano` pass.
4. Confirm `cargo check -p agent-diva-cli --no-default-features --features nano` and `cargo test -p agent-diva-cli --no-default-features --features nano` pass.
5. Confirm the older bootstrap log no longer overstates nano's independence level before this runtime decoupling change.
