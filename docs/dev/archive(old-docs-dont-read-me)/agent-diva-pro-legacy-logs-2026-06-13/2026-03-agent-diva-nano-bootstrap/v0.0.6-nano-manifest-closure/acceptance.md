# v0.0.6 Acceptance

## Acceptance Checks

1. `agent-diva-nano/Cargo.toml` no longer depends on workspace-inherited external crate versions.
2. The minimum internal dependency closure for nano is stated explicitly.
3. The remaining extraction blockers are listed explicitly.
4. Nano runtime decoupling from manager remains intact.
5. CLI nano mode semantics remain unchanged.

## User-Facing Conclusion

This round moves `agent-diva-nano` closer to a future starter/template split, but it does not claim that nano can already be moved out of the workspace safely.
