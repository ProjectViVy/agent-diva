# Story 2.2: Enforce Reviewer Assist Mode as True Read-Only Behavior

**Epic:** Epic 2 — Safe Capability Modes & Runtime Enforcement
**Status:** done
**Priority:** P1
**Depends on:** 2.1

## Story

As a DiVA user,
I want reviewer mode to be truly read-only,
So that code review and audit tasks cannot accidentally mutate files.

## Acceptance Criteria

- [x] AC1: Reviewer mask uses AgentMode::Assist
- [x] AC2: Write-capable tools excluded from tool exposure
- [x] AC3: Runtime rejects write tool calls in reviewer mode
- [x] AC4: GUI shows read-only status

## Tasks

- [x] Add AgentMode enum to MaskConfig
- [x] Implement read-only tool filtering in ToolPolicy
- [x] Add runtime enforcement in tool execution path
- [x] Update GUI to show reviewer mode indicator

## Dev Notes

- Not just prompt-only — runtime enforcement
- Write tools: terminal, write_file, patch

## File List

- `agent-diva-core/src/config/schema.rs` (modify)
- `agent-diva-agent/src/mask/tool_policy.rs` (modify)
- `agent-diva-agent/src/mask/mask_registry.rs` (modify)
- `agent-diva-agent/src/tool_assembly.rs` (modify)
- `agent-diva-agent/src/agent_loop.rs` (modify)
- `agent-diva-agent/src/agent_loop/loop_tools.rs` (modify)
- `agent-diva-agent/src/agent_loop/loop_turn.rs` (modify)
- `agent-diva-gui/src-tauri/src/commands.rs` (modify)
- `agent-diva-gui/src-tauri/src/lib.rs` (modify)
- `agent-diva-gui/src/composables/useMask.ts` (modify)
- `agent-diva-gui/src/components/MaskSwitcher.vue` (modify)
- `agent-diva-gui/src/locales/zh.ts` (modify)
- `agent-diva-gui/src/locales/en.ts` (modify)
- `workspace/masks/reviewer.md` (modify)

## Dev Agent Record

### Debug Log

- Hooked reviewer mask to `mode: assist` and synced allow/deny lists to read-only semantics.
- Added persisted active-mask state so runtime and GUI can read the same current mask.
- Filtered tool exposure at assembly time and rejected non-read tools again at execution time.
- Exposed Tauri mask commands and updated the GUI switcher to show read-only status.

### Completion Notes

- Reviewer mode is now enforced as a runtime read-only posture instead of prompt-only guidance.
- Validation passed for `agent-diva-agent` mask tests; GUI crate validation is still blocked by pre-existing sandbox compile errors and unrelated GUI typecheck warnings.

### File List

- `agent-diva-agent/src/mask/mask_registry.rs`
- `agent-diva-agent/src/mask/tool_policy.rs`
- `agent-diva-agent/src/tool_assembly.rs`
- `agent-diva-agent/src/agent_loop.rs`
- `agent-diva-agent/src/agent_loop/loop_tools.rs`
- `agent-diva-agent/src/agent_loop/loop_turn.rs`
- `agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva-gui/src-tauri/src/lib.rs`
- `agent-diva-gui/src/composables/useMask.ts`
- `agent-diva-gui/src/components/MaskSwitcher.vue`
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src/locales/en.ts`
- `workspace/masks/reviewer.md`

### Change Log

- Added persisted active mask state and restored it on startup.
- Introduced read-only tool filtering and runtime enforcement for reviewer mode.
- Added GUI mask metadata and read-only badge rendering.
- Wired Tauri mask commands for list/get/switch operations.
- Updated reviewer mask metadata to `assist` mode and read-only tool limits.

### Status

- done
