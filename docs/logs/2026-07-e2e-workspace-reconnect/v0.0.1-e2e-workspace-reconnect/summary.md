# Summary

- Reconnected `agent-diva-e2e` to the root Rust workspace so Cargo can resolve its `workspace = true` dependencies and package selection paths again.
- Added explicit `just` entrypoints for real-provider E2E execution instead of silently relying on `cargo test --all`.
- Kept live E2E opt-in: normal workspace test gates remain unchanged, while dedicated E2E commands surface the API key prerequisite clearly.
