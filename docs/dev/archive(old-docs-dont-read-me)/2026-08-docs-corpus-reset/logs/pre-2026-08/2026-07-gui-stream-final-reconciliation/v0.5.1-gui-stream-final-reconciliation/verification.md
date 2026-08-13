# Verification

- `cargo test -p agent-diva-agent --lib protocol_guard` passed (2 tests), including the new safe-suffix flush regression.
- `pnpm build` in `agent-diva-gui` passed (`vue-tsc --noEmit` and Vite production build).
- `cargo test -p agent-diva-agent protocol_guard` could not complete because pre-existing integration-test mocks do not implement the current `ToolChoiceMode` parameter in the provider trait.
- `cargo fmt --check -p agent-diva-agent` remains blocked by pre-existing formatting drift outside this change; no bulk formatting was applied.
