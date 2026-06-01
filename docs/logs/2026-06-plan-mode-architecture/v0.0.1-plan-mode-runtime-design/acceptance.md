# Acceptance

## User-Facing Acceptance Steps

- Open `docs/dev/agent-plan/plan-mode-architecture.md`.
- Confirm it defines Plan Mode as the only scope and excludes autodream, autonomous evolution, and long-running rhythm engines.
- Confirm it compares Codex, GenericAgent, and agent-diva responsibilities.
- Confirm it rejects GenericAgent magic files and Markdown regex state as the agent-diva source of truth.
- Confirm it defines `.agent-diva/plans/<plan-id>/` with `state.json`, `events.jsonl`, plan artifacts, verification, and step evidence.
- Confirm it defines approval and verification gates.
- Confirm `docs/dev/README.md` links to the new architecture document.

## Acceptance Result

Accepted when the above documentation checks pass and no code changes are included in the scoped diff.
