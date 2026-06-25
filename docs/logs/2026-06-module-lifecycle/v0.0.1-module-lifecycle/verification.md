# Verification

- `cargo fmt`
- `cargo test -p agent-diva-core -p agent-diva-tooling -p agent-diva-manager`
- `cargo test`
  - Blocked by a pre-existing `agent-diva-providers/src/litellm/client.rs` test compile failure: missing `Choice`, `ResponseMessage`, `ToolCall`, and `Function` imports in that crate's test module.
