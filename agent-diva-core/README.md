# agent-diva-core

`agent-diva-core` contains the shared foundation used across the Agent Diva
workspace: config loading, event bus types, session and memory services,
heartbeat/cron helpers, logging, and security primitives.

## Scope

- Shared domain types and error plumbing.
- Configuration schema and loaders.
- Event bus, session, memory, and heartbeat services.
- Cross-cutting runtime utilities consumed by higher-level crates.

This crate is intended to be stable enough for the rest of the `agent-diva-*`
publish chain, but it is still maintained as part of the workspace rather than
as a standalone framework.

## Minimal usage

```rust
use agent_diva_core::Result;

fn main() -> Result<()> {
    Ok(())
}
```

## Relationship to other crates

- `agent-diva-tooling` builds tool traits on top of core error/event types.
- `agent-diva-providers`, `agent-diva-tools`, and `agent-diva-agent` depend on
  this crate directly.
- `agent-diva-nano` consumes this crate through the higher-level publish chain.
