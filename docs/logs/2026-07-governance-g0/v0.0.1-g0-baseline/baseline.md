# G0 Structure and Performance Baseline

## Structure snapshot

Measured from the repository root with `(Get-Content <path>).Count`:

| File | Lines |
| --- | ---: |
| `agent-diva-agent/src/agent_loop/loop_turn.rs` | 2,570 |
| `agent-diva-agent/src/agent_loop.rs` | 3,103 |
| `agent-diva-manager/src/handlers.rs` | 1,484 |
| `agent-diva-manager/src/state.rs` | 395 |
| `agent-diva-gui/src/App.vue` | 2,295 |
| `agent-diva-gui/src/components/ChatView.vue` | 1,542 |
| `agent-diva-gui/src-tauri/src/commands.rs` | 6,525 |
| `agent-diva-gui/src/api/desktop.ts` | 841 |

Direct dependency counts from `cargo metadata --no-deps --format-version 1`:

- `agent-diva-agent`: 27
- `agent-diva-manager`: 29
- `agent-diva-gui`: 33
- `agent-diva-core`: 30

These figures are diagnostic baselines, not completion targets.

## Proposed repeatable timing method

Use a release build and a local Manager with a deterministic fake provider:

1. Record process start immediately before spawning the Manager.
2. Poll `/api/health` every 25 ms; record the first `200` or dependency-not-ready `502`.
3. Open chat SSE, record request dispatch, first data event, first tool-start event, matching tool-finish event, and final response.
4. Run 30 warm samples after 5 warmups for plain chat and a deterministic no-op tool call.
5. Report median and p95 for startup-to-health, request-to-first-event, and tool-start-to-tool-finish.

Normalize dynamic ports, IDs, timestamps, and temporary paths before storing output. This iteration records the method only: no samples were collected and no new performance threshold is enforced. G0.4 therefore remains open.
