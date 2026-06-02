# agent-diva-tooling

`agent-diva-tooling` defines the shared tool traits and registry primitives used
by Agent Diva runtimes and built-in tool implementations.

## Scope

- `Tool` trait for executable tool adapters.
- `ToolError` and `Result` types for tool-facing failures.
- `ToolRegistry` for registering and resolving tool instances.

This crate is the narrowest public contract in the tool stack. It is the right
dependency when you want to implement custom tools without pulling in the
built-in tool catalog.

## Minimal usage

```rust
use agent_diva_tooling::ToolRegistry;

fn main() {
    let _registry = ToolRegistry::new();
}
```

## Relationship to other crates

- `agent-diva-tools` provides concrete built-in tools that implement these
  traits.
- `agent-diva-agent` and `agent-diva-nano` use this crate to assemble toolsets.
