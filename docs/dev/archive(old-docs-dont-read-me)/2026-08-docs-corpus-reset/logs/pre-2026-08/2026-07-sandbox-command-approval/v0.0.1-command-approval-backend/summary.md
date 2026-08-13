# Iteration Summary

## Outcome

Completed the first backend phase of Sandbox command approval.

## Changes

- Production `exec` registration now routes through `ToolOrchestrator` with `OnFailure`.
- Recoverable sandbox escalation suspends the same tool call on a 300-second, session-scoped approval coordinator.
- Decisions support approve once, approve for session, and reject; timeout, cancellation, stale IDs, and missing clients fail closed.
- Manager shares the coordinator with Agent Loop and exposes pending-query, SSE request, and decision endpoints.
- Plan mode continues to exclude `exec`; stopping a chat cancels its pending approvals.
- Interactive tools may override the registry timeout so the approval deadline is not truncated by the default 120-second wrapper.

## Deferred

GUI consumption and persistent global safe-prefix rules remain in `TODOLIST.md` as Phase 2.
