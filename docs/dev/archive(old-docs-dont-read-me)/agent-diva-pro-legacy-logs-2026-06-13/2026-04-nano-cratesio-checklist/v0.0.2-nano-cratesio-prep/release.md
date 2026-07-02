# Release

## Status

No release was executed in this iteration.

## Recommended next release flow

1. Publish the dependency chain in order:
   `agent-diva-files -> agent-diva-core -> agent-diva-tooling -> agent-diva-providers -> agent-diva-tools -> agent-diva-agent -> agent-diva-nano`
2. After each publish, wait for crates.io index visibility before continuing.
3. Run `cargo package -p <crate> --allow-dirty` in a network-enabled
   environment before each actual publish.
