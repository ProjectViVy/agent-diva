# Story 3.2: Execute Isolated Parallel Child Agents

**Epic:** Epic 3 — Parallel Sub-Agent Orchestration
**Status:** ready-for-dev
**Priority:** P1
**Depends on:** 3.1

## Story

As a DiVA user,
I want child agents to run in parallel as isolated workers,
So that I can delegate multiple tasks without leaking identity.

## Acceptance Criteria

- [ ] AC1: Child agents run in parallel
- [ ] AC2: Each child has isolated execution context
- [ ] AC3: Children receive only task context, no personality
- [ ] AC4: Children don't communicate directly
- [ ] AC5: Partial failure returns all results

## Tasks

- [ ] Implement parallel spawn in SubagentManager
- [ ] Ensure context isolation per child
- [ ] Implement result collection with partial failure handling
- [ ] Add integration tests

## Dev Notes

- Use tokio::JoinSet for parallel execution
- Each child gets own ToolRegistry instance

## File List

- `agent-diva-agent/src/subagent.rs` (modify)