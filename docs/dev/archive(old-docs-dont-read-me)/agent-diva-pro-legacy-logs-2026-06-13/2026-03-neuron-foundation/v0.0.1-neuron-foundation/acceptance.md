# Acceptance

1. Workspace includes `agent-diva-neuron` and project compiles.
2. `agent-diva-neuron` exposes public APIs: `NeuronRequest`, `NeuronResponse`, `NeuronEvent`, `NeuronNode`, `LlmNeuron`, `NeuronError`.
3. `LlmNeuron` executes exactly one LLM turn and does not perform tool loops.
4. Existing `agent-diva-agent` / CLI behavior is unchanged.
5. Tests validate content path, tool-call passthrough, event emission, invalid input, and provider error mapping.
