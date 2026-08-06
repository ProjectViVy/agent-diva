# Memory Write-Path Layering Contract

- Status: Frozen contract (GA-MEM-PARITY Wave 0, 2026-08-06)
- Authority: `docs/logs/2026-08-05-memory-ga-parity-inventory/v0.0.1-gap-inventory/inventory.md`
  §10.1/§10.2/§10.3 (decisions 1–5, W1-1..W1-4, closure gaps G1–G4)
- Scope: responsibilities of every memory write path; who writes what, and
  which writes require governance.

## 1. Write paths and their responsibilities

| Path | Where it lives | Authority status | Who triggers it | Governance |
|------|----------------|------------------|-----------------|------------|
| **Working memory** | Session store (per-session, volatile) | Never authority | Agent (`update_working_checkpoint`, Wave 2) | None; session-scoped; distilled into long-term via `memory_distill` (G1) |
| **Long-term (low-risk add)** | Typed SQLite authority (`.laputa`) | Immediate authority | Agent `memory_add` for new facts (Wave 1) | Governed apply with audit; no HITL (decision 1, W1-2) |
| **Long-term (high-risk change)** | Proposal → typed authority | Proposal until approved | Agent `memory_update` / `memory_remove` (Wave 1) | Proposal + HITL; remove = tombstone, never physical delete |
| **L3 experience** | Skill file (`SKILL.md`, optional `kind: sop`) | Reusable artifact, not memory authority | Agent `memory_distill` (Wave 1) | New skill: immediate (audited); overwrite of existing skill: proposal (W1-3); per `docs/architecture/skill-sop-unification.md` |
| **AutoDream** | Proposal only (never direct authority) | Proposal until approved | AutoDream worker (scheduled/manual) | Reflection gate + proposal + HITL; candidate dedup vs applied authority (G4) |
| **Consolidation** | Same paths as long-term above | Fallback only | Internal, only when no explicit distill happened (decision 4) | Follows the long-term rules; itemized, not whole-file overwrite (E5) |

## 2. Priority and conflict rules

1. **Immediate apply wins over proposals**: a low-risk `memory_add` that is
   already applied takes precedence over a later AutoDream candidate with the
   same content — the AutoDream gate must skip or downgrade duplicates (G4).
   Realized in Wave 4 (2026-08-06): `LaputaService::applied_authority_digests`
   enumerates active AppliedAuthority record digests;
   `AutoDreamWorker::reflect` merges typed digests with legacy section digests
   into `BoundedReflectionInput.existing_memory_digests`; `CandidateGate`
   rejects same-content candidates as `CandidateRejectionCode::Duplicate`.
2. **Proposal never implies authority**: `SyncTurnStatus::ProposalCreated`
   means a reviewable proposal was durably stored; only
   `SyncTurnStatus::Persisted` means the authority write applied (W0-C).
3. **Working memory never becomes long-term implicitly**: promotion happens
   only through `memory_distill`, which carries the checkpoint summary as
   evidence (decision 3, G1).
4. **No silent fallback**: legacy Markdown and file writes are never promoted
   to authority implicitly; a degraded backend must be reported, not papered
   over (W0-B default Typed; F9).
5. **Consolidation demoted**: consolidation only fires when no explicit
   distill path was used; it must not double-write the same content (decision 4).
   Realized in Wave 5 (2026-08-07): `should_consolidate` checks
   `agent_diva_tools::distill_guard::distill_ran_this_session()`; `memory_distill`
   tool sets the flag on successful return. Consolidation prompt (v3) requires
   itemized `[{action, id?, content}]` output; each item is dispatched to
   `memory_add`/`memory_update`/`memory_remove` via `MemoryProvider` CRUD;
   non-array LLM output falls back to `sync_turn` proposal.

## 3. Honest status semantics

| Status | Meaning | Used by |
|--------|---------|---------|
| `Persisted` | Authority write applied; durable authority reflects the update | low-risk add, governed apply |
| `ProposalCreated` | Reviewable proposal durably created; not authority | high-risk update/remove, AutoDream, Laputa `sync_turn` |
| `Noop` | No write was needed | empty turns |
| `Failed { reason }` | Write attempted but did not complete | any path |

Wave 1 `memory` tools must surface the equivalent three states to the model:
`applied` / `proposal_created` (with proposal id) / `failed` (A9, §10.2).

## 4. Forgetting (tombstone) contract

- `memory_remove` produces a tombstone through proposal + approval; the
  tombstone is durable and auditable, never a physical delete (F7).
- Injection (startup/prefetch/context assembly) must recognize and filter
  tombstoned records so forgotten content stops appearing in prompts (G3).
- Same-session hot injection of applied changes is a Wave 3 concern (F4);
  Wave 1 only returns the new entry in the tool result (W1-4).

## 5. Working memory lifecycle (GC)

- Working memory records carry `scope.session_id`; they are volatile and
  session-scoped (never authority).
- `TypedLaputaMemoryProvider::on_session_end` physically deletes session-scoped
  records via `TypedMemoryStore::gc_session_scoped(session_id)`; failure is
  `warn!` only, never blocks session teardown.
- `TypedLaputaMemoryProvider::open` calls `gc_stale_session_scoped(active_ids)`
  to clean up orphan records from abnormal exits (crashes); `active_ids` comes
  from `SessionManager` enumeration; `None` skips GC (backward-compatible).
- Startup rendering already filters `session_id.is_some()` records (Wave 3 S1),
  so GC is a physical cleanup optimization, not a read-side correctness fix.

## 6. Known gaps

- **`memory_list` does not filter superseded records**: the `visible_record`
  predicate checks trust/tombstone/session but does not call
  `superseded_target_ids()`. `memory_search` and startup rendering do filter
  superseded records. Fix: wire `superseded_target_ids()` into `memory_list`
  (deferred to G2D+ / independent slice).

## 7. Cross-references

- Decisions: inventory §10.1 (1–5), §10.2 (W1-1..W1-4), §10.3 (G1–G4).
- Skill model: `docs/architecture/skill-sop-unification.md`.
- Typed authority architecture: `docs/architecture/laputa-memory-final-architecture.md`,
  `docs/architecture/memory-framework-interfaces.md`.
- Acceptance: `docs/logs/2026-08-05-memory-ga-parity-inventory/v0.0.1-gap-inventory/acceptance.md`.
