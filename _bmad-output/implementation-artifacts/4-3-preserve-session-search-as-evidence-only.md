---
baseline_commit: 63d1ea7
---

# Story 4.3: Preserve Session Search as Evidence Only

Status: ready-for-dev

## Story

作为用户，
我希望从报表和提案中搜索历史 session，
以便旧对话可以支持决策，但不会直接变成权威记忆。

## Acceptance Criteria

1. Given session search is available, when a report or proposal references search results, then results include session ID, timestamp, snippet, and evidence metadata.
2. Search results can be attached as `EvidenceRef`.
3. Search results are never injected into default runtime context as durable memory.
4. Corrupted session files are skipped with diagnostics.

## Tasks / Subtasks

- [ ] Add a read-only session search service over `agent-diva-core` session JSONL files, returning session ID, timestamp, bounded snippet, source URI, and diagnostics. (AC: 1, 4)
- [ ] Convert selected search hits into `EvidenceRef { source: Session, uri, excerpt, hash, created_at }` for report/proposal attachment. (AC: 2)
- [ ] Expose the search path to Report/Notebook and proposal creation without routing results through `MemoryProvider` as authority. (AC: 2, 3)
- [ ] Skip malformed/corrupted session files, collect per-file diagnostics, and continue scanning valid sessions. (AC: 4)
- [ ] Bound search cost: limit files scanned, snippets returned, snippet length, and total response bytes; add clear truncation metadata. (AC: 1)
- [ ] Add tests proving search results are evidence-only and do not appear in default prompt context unless later converted through an approved Laputa apply path. (AC: 3)

## Dev Notes

### Architecture Context

- `docs/architecture/evo-diva-architecture-2026-06-12.md` sections 7 and 11 define session search as evidence discovery, not authority.
- `docs/prd-report-system/prd.md` FR-9/FR-10 define v1 search as session-history traversal with structured results; later FTS/vector search is future work.
- Story 5.1 moved default runtime authority consumption behind applied Laputa reads; this story must not reintroduce session search into prompt authority.

### Current Code State

- `agent-diva-core/src/session/manager.rs` reads session JSONL and already skips unreadable/parsing failures in several list/load paths by returning `Option`/filtering entries.
- `SessionManager::save` is atomic from Story 1.6; search can assume complete replacement semantics but must still tolerate legacy/corrupt files.
- `agent-diva-core/src/evolution/types.rs` already has `EvidenceSource::Session` and `EvidenceRef`.
- Story 5.1 notes default prompts must exclude legacy files and unapplied evidence. Preserve that boundary.

### Implementation Guardrails

- This is not a memory write feature. Do not call `MemoryProvider::sync_turn`, Laputa apply, or direct authority file writes from search.
- Search output may feed `evidence_refs` for a proposal, but proposal creation still requires user action and later Evolution review/apply.
- Use structured JSON parsing for session lines. Avoid ad hoc mutation of session files.
- Do not fail the whole search because one session file is corrupt; report diagnostics and continue.
- Do not expose full session transcripts by default; return bounded snippets and stable evidence URIs.

### Testing Requirements

- Unit tests for:
  - successful search result shape with session ID, timestamp, snippet, evidence metadata;
  - corrupted JSONL skipped with diagnostics;
  - result limits and snippet truncation;
  - `EvidenceRef` conversion uses `EvidenceSource::Session`;
  - default runtime prompt/context excludes search hits unless applied through Laputa authority.
- Minimum validation:
  - targeted `cargo test -p agent-diva-core session`
  - targeted tests for any new report/proposal search API
  - `cargo test -p agent-diva-agent context` if prompt exclusion code is touched

### References

- `_bmad-output/planning-artifacts/epics.md` - Epic 4 Story 4.3.
- `docs/architecture/evo-diva-architecture-2026-06-12.md` - sections 7, 11, 13.5.
- `docs/prd-report-system/prd.md` - FR-9, FR-10, search evolution route.
- `_bmad-output/implementation-artifacts/1-6-make-session-saves-atomic.md` - atomic session save precondition.
- `_bmad-output/implementation-artifacts/5-1-plug-applied-laputa-reads-into-memoryprovider.md` - runtime authority boundary.
- `agent-diva-core/src/session/manager.rs` - current session loading/listing behavior.
- `agent-diva-core/src/evolution/types.rs` - `EvidenceRef` and `EvidenceSource::Session`.

## Previous Story Intelligence

- Story 4.2 creates proposals from Notebook actions. This story supplies optional session evidence for those proposals; it must not change proposal state or authority on its own.
- Story 5.1 explicitly excludes unapplied evidence from default prompt authority. If implementation touches prompt assembly, preserve its tests and behavior.

## Dev Agent Record

### Agent Model Used

TBD by implementation agent.

### Debug Log References

- 2026-06-15: Story context prepared from Epic 4, Report System PRD search requirements, session manager behavior, and Story 5.1 authority-boundary notes.

### Completion Notes List

- Ultimate context engine analysis completed - comprehensive developer guide created.

### File List

- `_bmad-output/implementation-artifacts/4-3-preserve-session-search-as-evidence-only.md`

### Change Log

- 2026-06-15: Created ready-for-dev story for evidence-only session search.
