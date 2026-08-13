# Wave C Remediation Summary

## Scope

- Close the remaining Wave C observability and readiness items that were still open in `TODOLIST.md`.
- Keep the patch limited to provider audit wiring, skill rejection audit payloads, JSONL sink behavior, manager readiness health, cron clock injection, and audit-page i18n.

## Changes

- Wrapped direct CLI provider construction in `ProviderTap` by lifting CLI provider builders to `Arc<dyn LLMProvider>`.
- Normalized skill upload rejection handling so malformed ZIPs, missing/invalid `SKILL.md`, and security decisions emit stable skill audit payloads using a deterministic skill name fallback.
- Reworked `JsonlAuditSink` with injected clock and writer-factory seams, explicit flush-on-emit visibility, deterministic day rolling, and swallowed write/flush/roll failures.
- Removed duplicate `"unknown"` skill rejection audit emission from low-level skill validation helpers so manager-level callers own stable payload emission.
- Changed `/api/health` from static liveness to readiness semantics with structured component state and `503` on degraded readiness.
- Added manager health state tracking for cron initialization and audit-sink readiness.
- Introduced a runtime clock seam for the manager monthly report cron callback.
- Moved audit-page, structured-events, and raw-log UI copy into `vue-i18n`, and added locale/test coverage.

## Outcome

- The Wave C backlog items for CLI provider audit tap, skill rejection audit coverage, JSONL sink visibility/rolling contract, cron clock abstraction, readiness health, and audit-page i18n are closed.
- A narrower follow-up remains in `TODOLIST.md` for health benchmark CI gating only.
