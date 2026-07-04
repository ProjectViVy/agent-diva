# Wave 2 Observability Remediation Summary

## Scope

- First remediation pass for Wave 2 / Wave C review findings.
- Close runtime blockers in audit sink wiring, `/api/logs`, manager audit parsing, cron audit sequencing, manager provider audit wiring, and tool-registry denial auditing.

## Changes

- Added workspace-local audit root resolution to manager state and created the directory during `AppState` initialization.
- Registered the JSONL audit sink before runtime tasks begin, and downgraded sink-init failure from process crash to logged fallback.
- Added JSONL audit timestamps so `/api/logs` can filter/paginate using persisted event time.
- Moved `/api/logs` to the same workspace audit root used by the sink and fixed empty-directory behavior.
- Replaced the fragile colon-delimited `/api/logs` cursor with a structured URL-safe base64 payload and strengthened pagination assertions.
- Filtered manager-side `gateway.log` parsing to `target == "audit"` so manager and GUI consume the same audit surface.
- Moved `CronJobStarted` emission after active-run ownership is secured to avoid orphan started events on re-entry.
- Emitted `ToolExecuted` audit records for `not_found` and `invalid_params` tool attempts.
- Wrapped manager runtime provider construction and hot-update provider swaps with `ProviderTap`.
- Added/updated focused regression tests for logs cursor behavior, audit-line filtering, cron service execution, and tool lookup failure.

## Outcome

- Wave 2 no longer blocks on the previously identified `/api/logs` cursor/path drift and missing manager runtime provider audit wiring.
- Remaining Wave 2 observability gaps were recorded in `TODOLIST.md` instead of being silently deferred.
