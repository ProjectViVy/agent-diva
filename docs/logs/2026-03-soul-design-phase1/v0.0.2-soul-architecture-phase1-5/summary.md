# Summary

## Iteration
- Version: `v0.0.2-soul-architecture-phase1-5`
- Scope: Complete the Soul architecture plan phases 1-5 with soft-governance.

## Changes
- `agent-diva-agent/src/context.rs`
  - Identity header is now loaded from `IDENTITY.md` first, with robust fallback to default identity.
  - Added parsing helpers for markdown identity fields (`Name`, `Emoji`, `Role`, `Style/Voice`).
  - Added tests for identity-file path, fallback path, empty identity, and parser behavior.
- `agent-diva-core/src/utils/mod.rs`
  - Upgraded default `BOOTSTRAP.md` template from checklist to dialogue-driven identity shaping flow.
- `agent-diva-cli/src/main.rs`
  - Improved onboarding completion message to explicitly describe soul initialization steps.
  - Threaded soul governance config into runtime tool configuration.
- `agent-diva-agent/src/agent_loop.rs`
  - Replaced boolean soul-change marker with a deduplicated per-turn changed-file set.
  - Added structured transparency notice with changed-file list and rationale.
  - Added soft governance hints:
    - Boundary confirmation suggestion when `SOUL.md` changes.
    - Frequent-change hint based on rolling-window turn count.
  - Added `SoulGovernanceSettings` runtime configuration.
  - Added tests for notice formatting and governance defaults.
- `agent-diva-agent/src/subagent.rs`
  - Extended subagent inherited persona context to include `USER.md`.
  - Updated tests to cover `USER.md` inheritance.
- `agent-diva-core/src/config/schema.rs`
  - Added configurable soul governance fields:
    - `frequent_change_window_secs`
    - `frequent_change_threshold`
    - `boundary_confirmation_hint`
  - Added serde defaults to keep backward compatibility.
- `agent-diva-core/src/config/validate.rs`
  - Added validation rules for new governance fields (`> 0` constraints).

## Impact
- User-visible identity is more file-driven and customizable.
- Soul updates are more transparent and explainable.
- Subagents better preserve main-agent/user persona consistency.
- Governance remains non-blocking, aligned with soft-constraint strategy.
