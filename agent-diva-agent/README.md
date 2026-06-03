# agent-diva-agent

`agent-diva-agent` implements the runtime agent loop used by Agent Diva. It
combines provider access, context assembly, skill flow, runtime control, and
tool configuration into the executable agent layer.

## Scope

- Agent loop orchestration and turn execution.
- Context assembly and consolidation helpers.
- Runtime control messages and subagent support.
- Built-in tool configuration and tool assembly.

This crate is higher-churn than the lower-level foundation crates because it
tracks runtime behavior closely. Public entry points should be treated as the
supported integration surface; internal scheduling details may still evolve.

## Minimal usage

```rust
use agent_diva_agent::BuiltInToolsConfig;

fn main() {
    let _config = BuiltInToolsConfig::default();
}
```

## Relationship to other crates

- Depends on `agent-diva-core`, `agent-diva-providers`, `agent-diva-tooling`,
  `agent-diva-tools`, and `agent-diva-files`.
- `agent-diva-nano` uses this crate as its runtime execution engine.
