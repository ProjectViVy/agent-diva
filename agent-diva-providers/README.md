# agent-diva-providers

`agent-diva-providers` contains LLM provider abstractions, discovery helpers,
catalog services, and concrete OpenAI-compatible integrations for Agent Diva.

## Scope

- Provider traits and request/response types.
- Registry and catalog helpers for resolving providers and models.
- Native-provider and LiteLLM-compatible client implementations.
- Model discovery and transcription-related support.

The crate is publishable on its own, but its compatibility bar is defined by the
Agent Diva runtime stack. Provider routing changes must preserve the project rule
that native OpenAI-compatible endpoints keep raw model identifiers.

## Minimal usage

```rust
use agent_diva_providers::ProviderRegistry;

fn main() {
    let _registry = ProviderRegistry::new();
}
```

## Relationship to other crates

- `agent-diva-agent` uses this crate for runtime model/provider execution.
- `agent-diva-nano` re-exports selected provider-facing entry points.
