# Iteration Summary

- Added a new workspace crate: `agent-diva-neuron`.
- Reserved non-looping neuron infrastructure for single-turn LLM calls.
- Introduced request/response contracts, neuron-local events, and a provider-backed default node.
- Kept existing `AgentLoop`, CLI behavior, and config schema unchanged.

## Impact Scope

- Additive only. No runtime behavior changes in existing channels, agent loop, or CLI paths.
