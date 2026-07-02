# Workspace Rustfmt Drift Acceptance

## Acceptance Steps

1. Run `cargo fmt --all -- --check`.
2. Run `just fmt-check`.
3. Inspect `agent-diva-agent/src/memory_boundary.rs` and confirm the `sync_turn` signature is wrapped per rustfmt defaults.
4. Inspect `TODOLIST.md` and confirm the rustfmt drift item is under Done.

## Expected Result

Workspace format validation passes without unrelated formatting blockers.
