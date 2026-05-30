# Summary: AutoDream Rhythm Distillation Design

> Date: 2026-05-30
> Version: v0.0.1-rhythm-distillation-design
> Status: Design specification completed

## What Changed

Produced the AutoDream rhythm distillation design specification for Agent-Diva: `docs/dev/genericagent/autodream-rhythm-distillation-design.md`.

This document defines:

- How AutoDream triggers (time gate, session count gate, manual, startup catch-up).
- Who executes it (background tokio task MVP, restricted subagent P1, manager job queue P2).
- What materials it reads (sessions, HISTORY.md, MEMORY.md, Source Capsules, Mentle, learning state, Journal history).
- What structured output it produces (memory_patch_candidates, journal_entries, learning_candidates, evidence_refs, confidence, review_required).
- What can auto-merge vs what requires user confirmation (tiered merge policy by section type and confidence).
- How it integrates with MEMORY.md, Journal, LearningCandidate, and Mentle.
- Lock/checkpoint strategy for concurrency control.
- Failure modes and graceful degradation.

## Impact Range

- New module: `agent-diva-core/src/autodream/` (checkpoint, lock, config, types).
- New module: `agent-diva-agent/src/autodream/` (worker, gates, prompt, evidence).
- Modified: `agent-diva-agent/src/agent_loop/loop_turn.rs` (session-end eligibility check).
- Modified: `agent-diva-manager/` (manual trigger endpoint).
- No changes to existing `MemoryProvider` trait.
- No changes to existing `consolidation.rs` behavior.
- No implementation code in this deliverable.
