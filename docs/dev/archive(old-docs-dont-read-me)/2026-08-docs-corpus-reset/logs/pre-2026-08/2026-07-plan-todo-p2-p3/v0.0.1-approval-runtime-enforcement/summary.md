# Plan/TODO P2 Review Remediation and P3 Runtime Enforcement

## Summary

Implemented revision-bound plan submission and approval persistence, then wired the capability policy into tool assembly and tool invocation. Approval remains user-originated; agents cannot self-approve or bypass submission by transitioning directly into approval or execution.

## Impact

- Submitted plans have a persisted revision and append-only submitted/approved events.
- Content changes after submission reopen the plan and invalidate the prior revision.
- Materialized execution TODO lists cannot be silently replaced.
- Plan-mode tool registries expose plan submission but no pre-approval TODO mutation.
- Every tool invocation is checked against the active persisted plan phase; unknown tools deny by default.
