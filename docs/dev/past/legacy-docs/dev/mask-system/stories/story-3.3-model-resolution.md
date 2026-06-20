# Story 3.3: Resolve Child Models and Runtime Limits Predictably

**Epic:** Epic 3 — Parallel Sub-Agent Orchestration
**Status:** ready-for-dev
**Priority:** P1
**Depends on:** 3.1

## Story

As a DiVA user,
I want child agents to use predictable model selection and execution limits,
So that delegation behavior stays controllable.

## Acceptance Criteria

- [ ] AC1: Explicit spawn model takes highest priority
- [ ] AC2: Fallback chain: child override → mask subagent_defaults → mask model → global
- [ ] AC3: Iteration/timeout limits enforced
- [ ] AC4: Status reflects timeout/cancellation correctly

## Tasks

- [ ] Implement model resolution chain in SubagentManager
- [ ] Add subagent_defaults support from mask config
- [ ] Implement timeout/iteration enforcement
- [ ] Add tests for resolution chain

## Dev Notes

- Model resolution: spawn > subagent_defaults > mask > global
- Use tokio::time::timeout for execution limits

## File List

- `agent-diva-agent/src/subagent.rs` (modify)