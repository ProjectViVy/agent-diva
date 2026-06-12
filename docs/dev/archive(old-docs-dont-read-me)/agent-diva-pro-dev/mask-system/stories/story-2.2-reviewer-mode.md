# Story 2.2: Enforce Reviewer Assist Mode as True Read-Only Behavior

**Epic:** Epic 2 — Safe Capability Modes & Runtime Enforcement
**Status:** ready-for-dev
**Priority:** P1
**Depends on:** 2.1

## Story

As a DiVA user,
I want reviewer mode to be truly read-only,
So that code review and audit tasks cannot accidentally mutate files.

## Acceptance Criteria

- [ ] AC1: Reviewer mask uses AgentMode::Assist
- [ ] AC2: Write-capable tools excluded from tool exposure
- [ ] AC3: Runtime rejects write tool calls in reviewer mode
- [ ] AC4: GUI shows read-only status

## Tasks

- [ ] Add AgentMode enum to MaskConfig
- [ ] Implement read-only tool filtering in ToolPolicy
- [ ] Add runtime enforcement in tool execution path
- [ ] Update GUI to show reviewer mode indicator

## Dev Notes

- Not just prompt-only — runtime enforcement
- Write tools: terminal, write_file, patch

## File List

- `agent-diva-core/src/config/schema.rs` (modify)
- `agent-diva-agent/src/mask/tool_policy.rs` (modify)