# v0.0.6 Nano Manifest Closure

## Summary

- Narrowed `agent-diva-nano`'s manifest coupling by replacing all `workspace = true` external dependencies with explicit crate versions in [`agent-diva-nano/Cargo.toml`](../../../../agent-diva-nano/Cargo.toml).
- Preserved the current internal dependency boundary:
  - required internal crates: `agent-diva-core`, `agent-diva-agent`, `agent-diva-providers`, `agent-diva-channels`, `agent-diva-tools`
  - not required for nano runtime: `agent-diva-manager`
- Kept internal crates on `path + version` intentionally so the remaining monorepo closure is explicit instead of hidden behind workspace inheritance.

## Minimum Dependency Closure For A Future Starter

`agent-diva-nano` currently needs this internal closure:

1. `agent-diva-core`
2. `agent-diva-providers`
3. `agent-diva-tools`
4. `agent-diva-agent`
5. `agent-diva-channels`
6. `agent-diva-nano`

Notes:

- `agent-diva-agent` itself depends on `agent-diva-core`, `agent-diva-providers`, and `agent-diva-tools`.
- `agent-diva-channels` depends on `agent-diva-core` and `agent-diva-providers`.
- `agent-diva-manager` is no longer in nano's runtime closure after `v0.0.5`.

## What Is Still Blocking Safe Extraction

- Internal crates above are still consumed through monorepo `path` dependencies.
- Those internal crates still need an explicit publish/move strategy:
  - publish to crates.io and consume by version
  - or move together into the future starter/template repository
- `agent-diva-cli` still consumes `agent-diva-nano` via monorepo `path` dependency in nano mode.
- No repository split/package workflow has been introduced yet for the nano starter line.

## Correct Status Statement

- Done: nano runtime is decoupled from manager runtime entrypoints.
- Done: nano manifest is less bound to workspace inheritance.
- Not done: nano is not yet safely extractable from this workspace.
