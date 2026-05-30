# Context Compaction Research — Iteration Summary

> Version: v0.0.1-context-compaction-design
> Date: 2026-05-31
> Status: Research document completed

## What Was Changed

New research document created: `docs/dev/genericagent/context-compaction-research.md`

This document provides a complete architecture design for session-local context compaction in Agent-Diva — the mechanism that keeps long sessions alive by summarizing older turns into a compact summary without polluting long-term memory (MEMORY.md).

## Scope of Research

1. **Claude Code reference analysis**: Deep reading of 8+ files in `.workspace/claude-code/src/services/compact/`, covering base compact, auto compact, micro compact, snip compact, reactive compact, session memory compact, prompt templates, grouping, post-compact cleanup, and boundary markers.

2. **Agent-Diva current state analysis**: Deep reading of `agent-diva-core/src/session/store.rs`, `manager.rs`, `agent-diva-agent/src/context.rs`, `agent-diva-agent/src/agent_loop/loop_turn.rs`, `agent-diva-agent/src/consolidation.rs`, `agent-diva-core/src/memory/provider.rs`, `agent-diva-tools/src/sanitize.rs`.

3. **Existing documentation review**: All 6 related documents in `docs/dev/genericagent/` and `docs/dev/archive/architecture-reports/`.

## Key Findings

1. Agent-Diva has **no session-local context compaction**. The only context management is message-count-based trimming (`get_history(50)`) and long-term memory consolidation (`consolidation.rs` → MEMORY.md).

2. Claude Code implements a sophisticated multi-layered compact system that is well worth borrowing patterns from, but should not be copied wholesale (GrowthBook, Ink REPL, cache_editing API, etc.).

3. `consolidation.rs` **cannot be repurposed** for context compaction — it writes to durable storage (MEMORY.md), its `last_consolidated` pointer has different semantics, and its output format is unsuitable.

4. The recommended architecture inserts a `ContextCompactor` between session history retrieval and prompt assembly, with a structured compact summary stored in `Session.metadata`.

5. Context compaction must maintain a hard boundary with `MemoryProvider` — it never writes to MEMORY.md and never calls `sync_turn()`.

## Impact Range

- No code changes in this iteration.
- Design affects future work on: `agent-diva-core/src/session/store.rs`, `agent-diva-agent/src/context.rs`, `agent-diva-agent/src/agent_loop/loop_turn.rs`, and new modules.
- `consolidation.rs` and `MemoryProvider` are explicitly NOT modified.

## Document Location

- Main report: `docs/dev/genericagent/context-compaction-research.md`
- README index updated: `docs/dev/genericagent/README.md`
