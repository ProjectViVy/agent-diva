# AutoDream Migration Research Summary

## Changed

- Added `docs/dev/genericagent/autodream-migration-research.md`.
- Updated `docs/dev/genericagent/README.md` to index the AutoDream migration research.

## Key Findings

- Claude Code has a real AutoDream implementation under `.workspace/claude-code/src/services/autoDream`.
- It supports automatic turn-end triggering and manual `/dream` triggering.
- Its reusable value for Agent-Diva is the runtime skeleton: gates, lock, background subagent, task progress, and extract/consolidate split.
- Agent-Diva should not copy Claude Code's narrow auto-memory semantics; it should adapt the mechanism into Diva rhythm, Journal, learning candidates, Laputa, and optional Mentle flows.

## Impact

- Documentation-only change.
- No runtime, configuration, or public API behavior changed.
