# Acceptance: AutoDream Rhythm Distillation Design

> Date: 2026-05-30
> Version: v0.0.1-rhythm-distillation-design

## Acceptance from User/Product Perspective

### What This Document Delivers

A design specification that allows an implementation team to directly break down AutoDream rhythm distillation into tasks without further architectural research.

### Key Questions Answered

1. **When does AutoDream trigger?**
   - MVP: Manual command + session-end eligibility check.
   - P1: Time gate (24h) + session count gate (5 sessions) + startup catch-up.
   - P2: Daily/weekly/monthly rhythm scheduling.
   - See Section 6.

2. **Who executes it?**
   - MVP: Background `tokio::spawn` task with restricted permissions.
   - P1: Restricted subagent via `SubagentManager`.
   - P2: Manager/service job queue with GUI status.
   - See Section 7.

3. **What materials does it read?**
   - Sessions (raw, not compact summaries), HISTORY.md, MEMORY.md, Source Capsules, Mentle recall, learning state, Journal history.
   - See Section 8.

4. **What does it produce?**
   - Structured `autodream_run.json` with: `memory_patch_candidates`, `journal_entries`, `learning_candidates`, `evidence_refs`, `confidence`, `review_required`.
   - See Section 9.

5. **What can auto-merge vs what needs user confirmation?**
   - Critical/sensitive sections (identity, relationship, rule): always require review.
   - Standard/low-risk sections: auto-merge only with explicit user policy and high confidence.
   - See Section 10.

6. **How does it integrate with MEMORY.md, Journal, LearningCandidate, Mentle?**
   - MEMORY.md: candidates only, no direct write by default.
   - Journal: natural output type, immutable entries with evidence refs.
   - LearningCandidate: candidate inbox for user-controlled learning.
   - Mentle: optional dense factual recall, not primary evidence source.
   - See Sections 10, 11, 12.

### What This Document Does NOT Deliver

- No implementation code.
- No GUI mockups (references existing UI design doc).
- No context compaction token algorithms.
- No provider model routing changes.
- No MemoryProvider trait refactoring.
- No replacement of existing consolidation.

## Acceptance Checklist

- [x] AutoDream is explicitly distinguished from context compaction.
- [x] Non-blocking guarantee for automatic triggers is specified.
- [x] Auto vs manual trigger differences are documented in a comparison table.
- [x] Lock/checkpoint strategy is specified with file format and stale recovery.
- [x] Output schema is fully specified with JSON examples.
- [x] MEMORY.md merge policy is tiered by section type and confidence.
- [x] Evidence chain traceability is documented.
- [x] Out-of-scope boundaries are explicitly listed.
- [x] MVP/P1/P2 phased plan is provided with concrete deliverables.
- [x] Open questions requiring product/architecture confirmation are listed.
