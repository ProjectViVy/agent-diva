# agent-diva-tooling

## OVERVIEW

Shared tool primitives: the `Tool` trait, `ToolError`/`Result`, `ToolRegistry`, and `Module`/`ModuleRegistry`. This is the narrowest public contract in the tool stack.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| `Tool` trait and error types | `src/base.rs` |
| Tool registry / deferred tools / schema partition | `src/registry.rs` |
| Module / bootstrap contract | `src/module.rs` |

## CONVENTIONS

- Keep this crate dependency-free of concrete tools, providers, channels, or the agent loop.
- New tool adapters implement `Tool` and are registered in a `ToolRegistry`.
- New runtime/module concepts extend `Module` and `ModuleRegistry` here.

## ANTI-PATTERNS

- Do not add built-in tool implementations here; put them in `agent-diva-tools`.
- Do not depend on `agent-diva-agent`, `agent-diva-providers`, or `agent-diva-channels` from this crate.
- Do not leak channel/provider-specific types through `ToolError`.

## NOTES

- `agent-diva-tools` implements concrete tools against these traits.
- `agent-diva-agent` uses this crate to assemble the per-turn tool registry.
