# Summary

## Changes

- Added the Sprint 4 A1 entry audit record for AgentLoop/Mentle hardening.
- Added regression coverage for active Mentle runtime startup routing, startup
  cron rebuild preservation, `with_toolset()` missing-status gating, and
  subagent Mentle isolation.
- Hardened subagent registry assembly so custom tools supplied to a parent-like
  assembly path are not carried into subagent mode.
- Updated Sprint 4 project-management status and validation evidence.

## Impact

- Main AgentLoop startup can prove `memtle_status` and L2 prompt routing become
  active together when runtime state is active.
- Cron/default rebuild paths keep the main agent's Mentle custom tools.
- Subagents default to no Mentle long-term memory capability at both config and
  registry assembly layers.
