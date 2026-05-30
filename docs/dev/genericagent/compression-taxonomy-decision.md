# Compression Taxonomy Decision for Agent-Diva

> Status: architecture decision note.
> Date: 2026-05-31.
> Scope: clarify the difference between context compaction, memory consolidation, and AutoDream rhythm distillation.

## 1. Decision

Agent-Diva should not use one overloaded meaning for "compression". The next design work should separate three mechanisms:

1. **Context compaction**: session-local context-window survival.
2. **Memory consolidation**: extracting durable memory candidates from conversation material.
3. **AutoDream rhythm distillation**: rhythmic refinement of Diva's long-term continuity, journal, and learning candidates.

Both context compaction and rhythm distillation are needed. They may share low-level summarization utilities, but they must not share the same semantic boundary, prompt contract, or output schema.

## 2. Why This Matters

The earlier compression research treated compression mainly as an AutoDream prerequisite. That remains partially correct, but it missed a separate and important class of compression: **context compaction**.

Context compaction is the kind of compression used when a single conversation grows too long and the model's context window becomes unstable. It keeps the current session alive by replacing older raw turns with a compact summary. This matches the behavior seen in Codex-style long conversations, where the agent can continue after repeated context compactions.

AutoDream rhythm distillation is different. It is not a short-term survival mechanism. It supports Diva's continuity by periodically reviewing evidence, sessions, memory, and user feedback, then producing refined memory candidates and journal material.

## 3. Mechanism Boundaries

### 3.1 Context Compaction

Purpose:

- Prevent a single session from exceeding the context budget.
- Preserve enough working state for the next turn to continue coherently.
- Avoid losing active files, active plans, recent decisions, unresolved questions, and current task state.

Lifecycle:

- Temporary and session-local.
- Triggered by token/message budget pressure.
- Stored as session state or a compact checkpoint.
- Used by prompt assembly instead of older raw session turns.

Hard boundary:

- It must not directly rewrite `MEMORY.md`.
- It must not claim durable truth.
- It should be treated as a lossy working summary, not as authoritative memory.

Likely owning area:

- `agent-diva-core/src/session/`
- `agent-diva-agent/src/context.rs`
- `agent-diva-agent/src/agent_loop/`

### 3.2 Memory Consolidation

Purpose:

- Convert useful session material into durable memory candidates.
- Extract stable facts, preferences, relationship updates, project state, and repeated behavioral patterns.

Lifecycle:

- Runs after turns or after sessions, depending on threshold and policy.
- Consumes raw session material, history, and optionally context compaction summaries.
- Produces structured candidates rather than blindly rewriting long-term memory.

Hard boundary:

- It may read current memory through `MemoryProvider`.
- It should not directly overwrite high-value memory without review or policy.
- Its output should be auditable and traceable to source turns or source capsules.

Likely owning area:

- Existing `agent-diva-agent/src/consolidation.rs`, but evolved away from direct `MEMORY.md` rewriting.
- `MemoryProvider::sync_turn()` remains the long-memory write boundary.

### 3.3 AutoDream Rhythm Distillation

Purpose:

- Support Diva's long-term subject continuity.
- Periodically refine experience into durable memory candidates, journal entries, and learning decisions.
- Preserve the philosophy that Diva's shared `MEMORY.md` is a continuity anchor, not a raw dump.

Lifecycle:

- Triggered by rhythm gates and manual trigger.
- May consume multiple sessions, history, capsules, prior memory, and user-confirmed learning candidates.
- Produces `memory_patch_candidate`, `journal_entry`, evidence links, confidence, and review flags.

Hard boundary:

- It should not be treated as context-window survival.
- It should not use context compaction output as sole evidence.
- It should write candidates first, then merge into `MEMORY.md` only by explicit policy or user review.

Likely owning area:

- Future AutoDream worker.
- Future rhythm scheduler.
- Candidate inbox / Journal integration.

## 4. Shared `MEMORY.md` Design

The user-facing design philosophy is that Diva should have one shared `memory.md` / `MEMORY.md` as the main continuity document. This is still a good direction.

However, a shared memory file should not imply full injection into every prompt. The right split is:

- **Authority**: `MEMORY.md` remains the human-readable continuity anchor.
- **Rendering**: prompt assembly should receive a compact rendered view from `MemoryProvider`.
- **Selection**: always include identity, relationship compass, durable commitments, and current high-priority project state; retrieve or render other sections only when relevant.
- **Archive**: lower-priority or evidence-heavy material should remain queryable, but not always injected.

This preserves Diva's continuity while preventing `MEMORY.md` from becoming a context-window liability.

## 5. Recommended Architecture

```text
Current turn
  -> SessionManager stores raw messages
  -> ContextBudgetMonitor estimates prompt pressure
  -> if needed: ContextCompactor creates session-local compact summary
  -> ContextBuilder renders prompt from:
       - system instructions
       - compact memory context
       - active session summary
       - recent raw turns

Turn/session end
  -> MemoryConsolidation extracts durable candidates
  -> Candidate inbox keeps evidence and review state

Rhythm/manual AutoDream
  -> AutoDream worker reads sessions/history/capsules/MEMORY.md
  -> produces memory patch candidates + journal entries + evidence refs
  -> policy/user review decides whether to merge into MEMORY.md
```

## 6. Implementation Guidance

Do first:

- Rename future design terms clearly: `context-compaction`, `memory-consolidation`, `autodream-distillation`.
- Implement context compaction as session-local and non-authoritative.
- Keep `MEMORY.md` as the continuity anchor, but render compact prompt views through `MemoryProvider`.
- Make AutoDream consume evidence and produce candidates, not direct memory rewrites.

Do not do:

- Do not use context compaction summaries as the only evidence for memory updates.
- Do not let rhythm distillation block the live response path.
- Do not collapse session-local compact summaries and durable memory candidates into the same file or schema.
- Do not make `MEMORY.md` both the raw archive and the prompt payload.

## 7. Consequence for Previous Compression Research

The earlier `compression-research.md` should be read as research for **AutoDream prerequisite material preparation**, not as the full compression architecture.

It needs one conceptual correction:

- "compression" there mostly means source capsule / memory preparation.
- Claude Code's compact mechanism mostly maps to **context compaction**.
- Agent-Diva needs both tracks, connected but separate.

