# agent-diva-neuron

## OVERVIEW

Single-turn, non-looping LLM execution primitives: request/response contracts, a neuron node trait, and a provider-backed executor. Used by the desktop GUI and future graph-level orchestration.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| Request/response types | `src/types.rs` (`NeuronRequest`, `NeuronResponse`) |
| Node trait | `src/node.rs` (`NeuronNode`, `NeuronError`) |
| Provider-backed executor | `src/executor.rs` (`LlmNeuron`) |
| Local event protocol | `src/events.rs` (`NeuronEvent`) |

## CONVENTIONS

- Keep this crate free of agent-loop orchestration; it is for single-turn calls only.
- New neuron executors implement `NeuronNode`.

## ANTI-PATTERNS

- Do not add multi-turn loop logic here; use `agent-diva-agent`.
- Do not depend on GUI-specific types from this crate.

## NOTES

- Currently used lightly by the GUI; kept small and stable for future workflow/emotion-system use.
