# Story 3.1: Define Batch Spawn and Structured Result Contracts

**Epic:** Epic 3 — Parallel Sub-Agent Orchestration
**Status:** ready-for-dev
**Priority:** P1
**Depends on:** 2.3

## Story

As a DiVA user,
I want child-agent spawning and result collection to use typed contracts,
So that parallel delegation is reliable and inspectable.

## Acceptance Criteria

- [ ] AC1: BatchSpawnRequest supports multiple child tasks with identity, goal, context
- [ ] AC2: SubAgentResult includes task_id, status, summary, elapsed, tokens, tool_trace
- [ ] AC3: Status distinguishes Ok, Error, Timeout, Cancelled
- [ ] AC4: Types live in shared config boundary

## Tasks

- [ ] Create SubAgentResult struct in agent-diva-core
- [ ] Create BatchSpawnRequest struct
- [ ] Add serialization/deserialization
- [ ] Add unit tests

## Dev Notes

- Types in agent-diva-core for cross-crate access
- Use serde for serialization

## File List

- `agent-diva-core/src/config/schema.rs` (modify)