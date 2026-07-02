# Story 3.4: Stream Child Lifecycle Events to Sub-Agent Panel

**Epic:** Epic 3 — Parallel Sub-Agent Orchestration
**Status:** ready-for-dev
**Priority:** P1
**Depends on:** 3.2

## Story

As a DiVA user,
I want to observe child-agent progress and outcomes in the GUI,
So that delegation remains understandable and trustworthy.

## Acceptance Criteria

- [ ] AC1: Runtime publishes typed events: spawned, progress, completed, failed, timeout, cancelled
- [ ] AC2: GUI panel renders child states without polling
- [ ] AC3: Final status shows duration and summary
- [ ] AC4: Empty state when no children active

## Tasks

- [ ] Define child lifecycle event types
- [ ] Implement event emission in SubagentManager
- [ ] Create GUI sub-agent panel component
- [ ] Wire Tauri IPC for event streaming

## Dev Notes

- Use Tauri event system for real-time updates
- Architecture A-6: Tauri IPC

## File List

- `agent-diva-agent/src/subagent.rs` (modify)
- `agent-diva-gui/src/components/SubAgentPanel.vue` (new)