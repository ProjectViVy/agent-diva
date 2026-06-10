# Story 2.3: Restrict Child Agents to Parent-Bounded Capabilities

**Epic:** Epic 2 — Safe Capability Modes & Runtime Enforcement
**Status:** ready-for-dev
**Priority:** P0
**Depends on:** 2.1

## Story

As a DiVA user,
I want child agents to stay within the capability boundary of the current session,
So that delegation never becomes a hidden privilege escalation path.

## Acceptance Criteria

- [ ] AC1: Child capabilities = subset of parent effective capabilities
- [ ] AC2: Child cannot access tools not available to parent
- [ ] AC3: child ⊆ parent always holds

## Tasks

- [ ] Modify `agent-diva-agent/src/subagent.rs` — use ToolPolicy for child creation
- [ ] Implement child capability resolution: parent ∩ child_allow − child_deny
- [ ] Add tests for parent/child combinations

## Dev Notes

- Architecture A-5: Independent ToolPolicy, runtime resolution
- SubagentManager creates child with reduced ToolRegistry

## File List

- `agent-diva-agent/src/subagent.rs` (modify)
- `agent-diva-agent/src/mask/tool_policy.rs` (modify)