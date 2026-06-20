# Story 2.1: Compute Effective Capabilities from Mask Policy

**Epic:** Epic 2 — Safe Capability Modes & Runtime Enforcement
**Status:** ready-for-dev
**Priority:** P0
**Depends on:** 1.1

## Story

As a DiVA user,
I want mask tool policies to produce a clear effective capability set,
So that each mask exposes only the tools it is meant to use.

## Acceptance Criteria

- [ ] AC1: Effective tools = global_builtin ∩ allow − deny
- [ ] AC2: Unknown tools in allow/deny are ignored gracefully
- [ ] AC3: Only effective tools exposed to model
- [ ] AC4: Deterministic computation

## Tasks

- [ ] Create `agent-diva-agent/src/mask/tool_policy.rs` — ToolPolicy struct
- [ ] Implement `resolve()` method: global ∩ allow − deny
- [ ] Add unit tests for various allow/deny combinations
- [ ] Export ToolPolicy for use by ToolRegistry

## Dev Notes

- Architecture A-4: ToolRegistry level filtering
- Architecture A-5: Independent ToolPolicy
- Use HashSet for tool name sets

## File List

- `agent-diva-agent/src/mask/tool_policy.rs` (new)