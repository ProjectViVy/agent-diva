# HQ-05 Release

The Session Admission Epic is release-ready and committed locally on `dev`. It has not been pushed,
tagged, packaged, or deployed externally.

## Configuration and migration

The additive `agents.defaults.session_admission` object requires no manual migration. Missing values
receive `max_queue_depth=2`, `wait_timeout=30`, and `idle_ttl=600`; durations are seconds. Queue depth
may be zero, while both durations must be greater than zero. Operators should validate edited JSON
and restart the process that owns the Agent Loop.

The authoritative operating and migration instructions are in
`docs/dev/harness-session-admission/operator-guide.md`.

## Rollback

No database or persisted-session rollback is required. The safest emergency whole-Epic rollback is
to stop new traffic, wait for accepted turns to quiesce, and deploy baseline `8b901d4f`. An older
binary ignores the additive configuration object. Source-revert order and partial-revert constraints
are documented in the operator guide.

## Release decision

All automated, focused, CLI, GUI, and embedded-Gateway gates passed. HQ-00 through HQ-05 are closed;
there is no remaining Session Admission TODO or required manual blocker.
