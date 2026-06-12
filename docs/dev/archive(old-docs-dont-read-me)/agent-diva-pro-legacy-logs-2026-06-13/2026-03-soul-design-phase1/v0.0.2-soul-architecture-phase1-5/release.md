# Release

## Release Type
- Internal development release (workspace code update, no external deployment pipeline required).

## Deployment Method
- Not applicable for this iteration.
- Reason: this iteration modifies local workspace behavior and validation coverage; no production release step is defined in the repository for this scope.

## Rollout Notes
- Backward compatibility is preserved:
  - Identity loading falls back to default header when `IDENTITY.md` is missing or empty.
  - New soul governance config fields have defaults and validation guards.
- No destructive migration required.

## Rollback Plan
- Revert modified files in:
  - `agent-diva-agent/src/context.rs`
  - `agent-diva-agent/src/agent_loop.rs`
  - `agent-diva-agent/src/subagent.rs`
  - `agent-diva-cli/src/main.rs`
  - `agent-diva-core/src/utils/mod.rs`
  - `agent-diva-core/src/config/schema.rs`
  - `agent-diva-core/src/config/validate.rs`
- Re-run `just fmt-check && just check && just test` after rollback.
