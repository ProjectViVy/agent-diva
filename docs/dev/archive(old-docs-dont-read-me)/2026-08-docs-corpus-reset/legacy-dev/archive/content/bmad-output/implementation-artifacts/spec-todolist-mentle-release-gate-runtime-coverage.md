---
title: 'TODOLIST Mentle release-gate runtime coverage'
type: 'chore'
created: '2026-06-18'
status: 'done'
route: 'one-shot'
context: []
---

# TODOLIST Mentle release-gate runtime coverage

## Intent

**Problem:** `TODOLIST.md` tracked that the Story 6.5 release-gate test only validated static Mentle tool filtering and did not exercise enabled-runtime governance prompt assembly.

**Approach:** Replace the gate with AgentLoop-level enabled-runtime tests, expose a read-only prompt builder on `AgentLoop`, then move the verified TODO to Done with iteration logs.

## Suggested Review Order

- `../../agent-diva-agent/tests/mentle_governance_boundaries.rs` -- verify the release-gate test now builds enabled Mentle toolsets and asserts governance prompt exclusion.
- `../../agent-diva-agent/src/agent_loop.rs` -- verify the new helper is read-only and delegates to the configured context.
- `../../TODOLIST.md` -- verify only the completed Mentle item moved from Open to Done.
- `../../docs/logs/2026-06-todolist-closeout/v0.0.2-mentle-release-gate-runtime-coverage/verification.md` -- verify the command results and deferred rustfmt note.
