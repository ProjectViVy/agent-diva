# agent-diva-tools

`agent-diva-tools` bundles the built-in tool implementations shipped with Agent
Diva, including filesystem, shell, web, cron, spawn, attachment, and MCP-backed
tools.

## Scope

- Concrete built-in tools on top of `agent-diva-tooling`.
- MCP SDK integration and tool discovery helpers.
- Utility exports for tool registration and sanitization.

This crate is suitable for runtimes that want the default Agent Diva tool set.
If you only need the public traits for custom tools, depend on
`agent-diva-tooling` instead.

## Minimal usage

```rust
use agent_diva_tools::ToolRegistry;

fn main() {
    let _registry = ToolRegistry::new();
}
```

## Relationship to other crates

- `agent-diva-tooling` provides the core traits used here.
- `agent-diva-agent` and `agent-diva-nano` consume this crate to assemble
  end-user tool availability.
