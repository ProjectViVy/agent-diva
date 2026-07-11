# Release

- **Ship with:** next gateway / CLI rebuild that includes `agent-diva-core` and `agent-diva-manager`.
- **Requires restart:** yes — process-local registry only; running gateway must be restarted to load the fix.
- **Migration:** none (ephemeral in-memory state; no DB schema).
- **Rollback:** revert the `Default`/`PlanningService::new` changes if unexpected cross-component sharing appears (unlikely; process-wide share was the intended design).
