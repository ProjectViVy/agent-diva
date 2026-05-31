# Context Compaction vs Autonomous Evolution Decision

> Status: accepted architecture direction.
> Date: 2026-05-31.
> Scope: decide the boundary and implementation order between session-local context compaction and Agent-Diva autonomous evolution.

## 1. Decision

Agent-Diva should treat **context compaction** and **autonomous evolution** as two separate architecture tracks.

The current main goal is **autonomous evolution**, not context compaction.

Context compaction remains important, but it should be positioned as a runtime survival mechanism for long single sessions. It should not become the semantic center of AutoDream, memory refinement, or Diva's subject continuity.

Implementation should start with the autonomous evolution foundation, while reserving clean extension points for context compaction:

1. Build the candidate, evidence, changelog, audit, and review spine first.
2. Build Shared `MEMORY.md` rendering so long memory can enter prompts safely.
3. Build manual AutoDream / rhythm distillation on top of that spine.
4. Add minimal context budget monitoring early, but defer full context compaction until the evolution pipeline has a stable write boundary.

## 2. Boundary

### 2.1 Context Compaction

Context compaction is a **session-local survival mechanism**.

It answers this question:

> How does the current conversation continue safely when the prompt becomes too large?

It should:

- watch context pressure inside a single session;
- summarize or replace older prompt history only when needed;
- preserve active task state, active files, decisions, unresolved questions, and current plan state;
- store a compact checkpoint as session runtime state;
- feed prompt assembly with a shorter working history.

It should not:

- directly write `MEMORY.md`;
- create durable learning facts;
- bypass candidate review;
- become the only source of evidence for memory updates;
- be treated as AutoDream.

### 2.2 Autonomous Evolution

Autonomous evolution is a **cross-session continuity mechanism**.

It answers this question:

> How does Diva gradually refine long-term memory, relationship continuity, project continuity, journal material, and learning candidates from evidence?

It should:

- read sessions, `HISTORY.md`, source capsules, rendered `MEMORY.md`, and explicit user feedback;
- run by manual trigger and later by rhythm trigger;
- produce structured candidates rather than directly rewriting authority files;
- attach evidence references to every proposed durable change;
- route memory changes through review, changelog, audit, and rollback;
- keep `MEMORY.md` as the human-readable continuity anchor.

It should not:

- depend on context compaction as its primary source of truth;
- block the live response path;
- auto-merge high-impact memory changes in early versions;
- confuse lossy prompt summaries with durable memory evidence.

## 3. Priority

The recommended order is:

1. **Autonomous evolution foundation first.**
   - Define `EvidenceRef`, candidate models, candidate states, changelog records, audit events, and rollback requirements.
   - This creates the safety boundary needed before any model-generated memory proposal can become durable state.

2. **Shared `MEMORY.md` rendering next.**
   - Keep `MEMORY.md` as authority.
   - Render only the prompt-safe subset.
   - Use Always / Relevant / Archive-style layering so memory does not overwhelm context.

3. **Manual AutoDream / rhythm distillation P0a.**
   - Manual trigger first.
   - Read evidence.
   - Produce candidates and journal drafts.
   - Do not directly merge into `MEMORY.md`.

4. **Minimal context budget instrumentation in parallel.**
   - Add token or message pressure estimation.
   - Add clear prompt assembly boundaries.
   - Add metadata fields/checkpoints needed by future compaction.
   - Avoid implementing a full compactor before the memory governance spine is stable.

5. **Full context compaction after the evolution write path is safe.**
   - Once prompt rendering and candidate governance are stable, add session-local compaction.
   - Keep it isolated from long-memory writes.

## 4. Why Not Start With Full Context Compaction

Starting with full context compaction would solve a real runtime problem, but it does not solve the core product question of this cycle.

The core product question is not:

> Can a single long session keep talking after the prompt grows too large?

The core product question is:

> Can Diva produce trustworthy long-term self-continuity from evidence, without corrupting memory or silently inventing durable facts?

That requires the candidate, evidence, review, changelog, audit, rollback, and rendering layers first.

Context compaction can help later by keeping long sessions stable. It can also provide extra material to AutoDream as a secondary evidence source. But it should not be the foundation of autonomous evolution.

## 5. Minimal Early Work for Context Compaction

Although full context compaction should not lead the work, the architecture should reserve these early hooks:

- `ContextBudgetMonitor`: estimates prompt pressure before prompt assembly.
- `PromptHistoryPolicy`: decides which raw turns, summaries, memory render, and active task state enter the prompt.
- `SessionCompactionCheckpoint`: records whether a session has a compact working summary.
- `CompactionEvidenceRef`: allows future AutoDream to reference a compaction summary as secondary evidence, never sole evidence.

These hooks keep the later compaction design from fighting the autonomous evolution architecture.

## 6. Formal Architecture Principle

Use this principle in future design reviews:

> Context compaction keeps the current conversation alive. Autonomous evolution keeps Diva continuous across time. They may share low-level summarization utilities, but they must not share authority, lifecycle, or write semantics.

## 7. Consequences

For planning:

- Do not block AutoDream P0 on full context compaction.
- Do block AutoDream memory writes on candidate review, changelog, and rollback design.
- Treat context compaction as an important runtime feature, but not as the main path of this iteration.

For implementation:

- Keep compaction code near session/prompt assembly.
- Keep autonomous evolution code near rhythm, candidates, memory governance, and journal/audit.
- Never let a compact prompt summary directly overwrite `MEMORY.md`.

For documentation:

- `context-compaction-research.md` remains the design reference for session-local prompt survival.
- `autodream-rhythm-distillation-design.md` remains the design reference for rhythm memory distillation.
- `candidate-evidence-journal-audit-design.md` should be treated as the governing write-path specification for autonomous evolution.
- `shared-memory-rendering-research.md` should be treated as the bridge between authority memory and prompt-safe memory context.

