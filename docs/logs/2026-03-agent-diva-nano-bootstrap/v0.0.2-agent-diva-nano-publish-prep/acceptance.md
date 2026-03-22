# Acceptance

1. Confirm `agent-diva-nano/src/` now contains local copies of:
   - `handlers.rs`
   - `manager.rs`
   - `mcp_service.rs`
   - `server.rs`
   - `skill_service.rs`
   - `state.rs`
2. Confirm [`agent-diva-nano/src/lib.rs`](../../../agent-diva-nano/src/lib.rs) no longer uses cross-crate `#[path = "../../agent-diva-manager/src/..."]`.
3. Confirm the default `full` CLI path still uses the original inlined `run_gateway`.
4. Confirm the `nano` CLI path still builds via `--no-default-features --features nano`.
5. Confirm `agent-diva-core` can be packaged locally and `agent-diva-nano` package attempts now fail only because upstream crates are not yet published on crates.io.
