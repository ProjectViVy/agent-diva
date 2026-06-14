---
title: "EVO-DIVA Architecture - Unified Autonomous Evolution Contract"
status: final
created: 2026-06-12
updated: 2026-06-12
version: 1.0.0
project: agent-diva-pro
supersedes:
  - _bmad-output/planning-artifacts/architecture.md
related_prds:
  - _bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md
  - docs/prds/prd-laputa-2026-06-12/prd.md
  - docs/prds/prd-selfinprove-2026-06-12/prd.md
  - docs/prds/prd-autodream-2026-06-12/prd.md
  - docs/prd-report-system/prd.md
related_decisions:
  - _bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/.decision-log.md
  - docs/architecture/scope-merge-decision.md
  - docs/architecture/autodream-architecture-2026-06-12.md
---

# EVO-DIVA Architecture

This document is the implementation architecture for EVO-DIVA. It supersedes the
old VRM-merge architecture for this epic. The old architecture remains useful for
workspace stack constraints, but it is not the autonomous-evolution authority
contract.

The governing rule is:

> GenericAgent runs Diva. Laputa owns authority. AutoDream reflects and proposes.
> SelfImprove makes governance visible. Report System consumes and solidifies.
> Mentle remains existing tool integration only. Context Compaction stays
> session-local.

The architecture starts from the authority spine, not from UI or background
automation:

```text
EvidenceRef
  -> EvolutionProposal
  -> user review
  -> Laputa apply
  -> changelog / audit
  -> rollback
  -> prompt / report consumption
```

Any durable subject, memory, SOP, skill, identity, relationship, commitment,
preference, policy, or report-derived authority change that bypasses this spine
is a P0 architecture violation.

## 1. Scope And Overrides

### 1.1 Source Of Truth

This architecture binds five planning contracts:

- Governance PRD:
  `_bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md`
- Laputa PRD:
  `docs/prds/prd-laputa-2026-06-12/prd.md`
- AutoDream PRD:
  `docs/prds/prd-autodream-2026-06-12/prd.md`
- SelfImprove PRD:
  `docs/prds/prd-selfinprove-2026-06-12/prd.md`
- Report System PRD:
  `docs/prd-report-system/prd.md`

Governance decision D-009 overrides older references that propose deeper Mentle
participation in EVO-DIVA v1. Mentle remains compatibility-only in this epic.

### 1.2 Preserved Platform Constraints

The workspace stack remains the stack locked by the old architecture:

- Rust 2021 workspace.
- Vue 3 + Vite + Tauri v2 GUI.
- Existing workspace members stay intact.
- Existing feature-gated Mentle integration remains available.
- Sandbox crate behavior is preserved.

EVO-DIVA adds new crates and contracts; it does not reopen the VRM merge plan.

### 1.3 Non-Goals For v1

EVO-DIVA v1 does not add:

- silent auto-merge for durable changes;
- a second authority store in Mentle;
- Mentle report synchronization, AutoDream indexing, audit, rollback, or default
  prompt injection;
- Context Compaction writes to Laputa or long-term memory;
- monthly-report generation inside AutoDream;
- broad tool access for AutoDream runs;
- external `laputa_core` path imports.

## 2. Authority Spine

### 2.1 Required Flow

All durable evolution follows this sequence:

1. Evidence is captured as `EvidenceRef`.
2. A producer creates an `EvolutionProposal`.
3. SelfImprove or an equivalent user-visible surface exposes the proposal.
4. The user approves, rejects, edits, or marks it for attention.
5. Laputa validates and applies the approved proposal.
6. Laputa writes changelog and audit records atomically with the section change.
7. Rollback can restore eligible changes.
8. Prompt rendering and Report System reads consume the updated authority state.

No producer is allowed to convert evidence directly into authority writes.

### 2.2 Producers And Consumers

| Component | Produces | Consumes | Authority Writes |
|---|---|---|---|
| GenericAgent | sessions, tool events, runtime evidence | Laputa prompt adapter output | no |
| AutoDream | run records, proposals, daily/weekly reports | sessions, Laputa snapshots/sections | no |
| SelfImprove | user decisions, edits, manual triggers | proposals, changelog, diagnostics | only through Laputa apply |
| Report System | monthly reports, report proposals, search evidence | AutoDream reports, Laputa reads | only through Laputa apply |
| Laputa | authority snapshot, changelog, audit, rollback events | approved proposals | yes |
| Mentle | existing tool outputs | existing tool config/runtime | no EVO-DIVA role |
| Context Compaction | session-local summaries | current session context | no |

### 2.3 Direct-Write Ban

Only `agent-diva-laputa` may write `.laputa/` authority state. Other crates must
use Rust APIs, manager routes, or Tauri commands that call Laputa.

Forbidden v1 patterns outside `agent-diva-laputa` include:

```text
fs::write(... ".laputa" ...)
std::fs::write(... "MEMORY.md" ...)
std::fs::write(... "SOUL.md" ...)
std::fs::rename(... ".laputa" ...)
```

Compatibility migration code may read legacy files, but durable writes must be
converted into proposals or internal Laputa migrations.

## 3. Workspace Shape

### 3.1 New Crates

Add two workspace crates:

```text
agent-diva-laputa/
agent-diva-autodream/
```

Update the root `Cargo.toml` workspace members:

```toml
members = [
  # existing members...
  "agent-diva-laputa",
  "agent-diva-autodream",
]
```

### 3.2 Existing Crates To Extend

| Crate | EVO-DIVA Responsibility |
|---|---|
| `agent-diva-core` | shared governance domain types and stable `MemoryProvider` adapter boundary |
| `agent-diva-laputa` | authority storage, proposal persistence, apply, changelog, audit, rollback, migration |
| `agent-diva-autodream` | manual runs, locks, checkpoints, restricted prompt execution, proposal/report outputs |
| `agent-diva-manager` | HTTP/SSE/Tauri-facing orchestration endpoints that call Laputa and AutoDream |
| `agent-diva-gui/src-tauri` | Tauri command exposure for GUI |
| `agent-diva-gui` | Evolution workspace, Notebook proposal migration, Chat trigger, Settings policy |
| `agent-diva-agent` | keep GenericAgent runtime and existing Mentle integration; no EVO-DIVA authority writes |

## 4. Shared Domain Model

Add shared governance types in `agent-diva-core`, preferably under:

```text
agent-diva-core/src/evolution/mod.rs
agent-diva-core/src/evolution/types.rs
```

The first shared model must include:

- `EvidenceRef`
- `EvidenceSource`
- `EvolutionProposal`
- `ProposalType`
- `ProposalState`
- `LaputaSectionName`
- `ChangelogRecord`
- `AuditEvent`
- `RollbackRequest`
- `AutoDreamRunRecord`

### 4.1 EvidenceRef

`EvidenceRef` is a typed pointer to evidence. It must not embed unbounded session
or file content by default.

Required fields:

```rust
pub struct EvidenceRef {
    pub id: String,
    pub source: EvidenceSource,
    pub uri: String,
    pub excerpt: Option<String>,
    pub hash: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}
```

`EvidenceSource` variants:

- `Session`
- `Report`
- `AutoDreamRun`
- `LaputaSection`
- `UserInput`
- `File`
- `ContextCompaction`

Compaction evidence is secondary only. It cannot be the sole evidence for a
durable authority proposal.

### 4.2 EvolutionProposal

Required fields:

```rust
pub struct EvolutionProposal {
    pub id: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub created_by: String,
    pub proposal_type: ProposalType,
    pub target_section: LaputaSectionName,
    pub evidence_refs: Vec<EvidenceRef>,
    pub proposed_patch: String,
    pub risk_level: RiskLevel,
    pub state: ProposalState,
    pub source_run_id: Option<String>,
}
```

`ProposalType` variants:

- `MemoryPatch`
- `JournalNote`
- `LearningNote`
- `IdentityPatch`
- `RelationshipUpdate`
- `CommitmentSet`
- `SopCreate`
- `Deprecation`

Unknown proposal types must fail before write validation.

### 4.3 Proposal State Machine

Allowed states:

- `PendingReview`
- `Approved`
- `Rejected`
- `Edited`
- `Applied`
- `Reverted`
- `Superseded`
- `NeedsAttention`
- `RunFailed`

Minimum transition rules:

| From | Allowed To |
|---|---|
| `PendingReview` | `Approved`, `Rejected`, `Edited`, `NeedsAttention`, `Superseded` |
| `Edited` | `Approved`, `Rejected`, `Edited`, `NeedsAttention`, `Superseded` |
| `Approved` | `Applied`, `Rejected`, `NeedsAttention` |
| `Applied` | `Reverted`, `Superseded` |
| `NeedsAttention` | `PendingReview`, `Rejected`, `Edited`, `Superseded` |
| `RunFailed` | terminal for the failed run record |
| `Rejected`, `Reverted`, `Superseded` | terminal unless explicitly copied into a new proposal |

Invalid transitions return typed errors and, when user-visible, produce audit
events.

## 5. Laputa Architecture

### 5.1 Crate Role

`agent-diva-laputa` is the only v1 authority implementation. It is file-first,
in-repo, and owns `.laputa/` state.

It must implement:

- file-first authority storage under `.laputa/`;
- proposal persistence;
- apply validation;
- changelog creation;
- audit creation;
- rollback;
- schema migration;
- section reads;
- direct-write prevention tests.

### 5.2 Storage Layout

Initial storage:

```text
.laputa/
  state.json
  proposals/
    pending/
    applied/
    rejected/
    attention/
  changelog/
    records.jsonl
    details/
  audit/
    events.jsonl
  rollback/
    staging/
  migrations/
  locks/
  legacy/
```

`state.json` is the canonical v1 snapshot. It contains `schema_version` and the
14 Laputa sections defined by the Laputa PRD. TBD sections are represented with
explicit `status = "tbd"` metadata, never by missing fields.

### 5.3 Atomic Writes

Laputa writes must use temp-file plus durable rename:

```text
write .tmp
fsync file where supported
rename .tmp -> final
fsync parent dir where supported
```

Composite operations use staging markers. If a process exits mid-apply or
mid-rollback, startup recovery must either complete or revert the staged
operation before accepting new writes.

### 5.4 Locking

Laputa uses OS-level file locking for section writes:

- same section writes are serialized;
- different section writes may run concurrently if state-file atomicity remains
  correct;
- lock wait timeout defaults to 5 seconds;
- stale/deadlock recovery produces `NeedsAttention` audit events.

### 5.5 Rust API

Expose a domain API from `agent-diva-laputa`; manager HTTP/Tauri routes wrap this
API.

Required operations:

```rust
pub trait LaputaStore {
    fn create_proposal(&self, proposal: EvolutionProposal) -> Result<EvolutionProposal>;
    fn list_proposals(&self, filter: ProposalFilter) -> Result<Vec<EvolutionProposal>>;
    fn update_proposal_state(
        &self,
        id: &str,
        transition: ProposalTransition,
    ) -> Result<EvolutionProposal>;
    fn apply_proposal(&self, id: &str, actor: ActorRef) -> Result<ChangelogRecord>;
    fn read_snapshot(&self) -> Result<LaputaSnapshot>;
    fn read_section(&self, name: LaputaSectionName) -> Result<LaputaSection>;
    fn list_changelog(&self, filter: ChangelogFilter) -> Result<Page<ChangelogRecord>>;
    fn read_changelog_detail(&self, id: &str) -> Result<ChangelogDetail>;
    fn rollback_changelog(
        &self,
        id: &str,
        request: RollbackRequest,
        actor: ActorRef,
    ) -> Result<ChangelogRecord>;
}
```

### 5.6 Manager And Tauri API

Expose the Rust contract through manager HTTP and GUI/Tauri commands.

Required Laputa operations:

- create/list/update proposals;
- apply proposal;
- read snapshot;
- read section;
- list changelog;
- read changelog detail;
- rollback changelog;
- poll/SSE proposal events;
- poll/SSE changelog events;
- poll/SSE needs_attention/error events.

Route shape:

```text
POST   /api/laputa/proposals
GET    /api/laputa/proposals
PATCH  /api/laputa/proposals/{id}
POST   /api/laputa/proposals/{id}/apply
GET    /api/laputa/snapshot
GET    /api/laputa/section/{name}
GET    /api/laputa/changelog
GET    /api/laputa/changelog/{id}
POST   /api/laputa/changelog/{id}/rollback
GET    /api/laputa/events/proposals
GET    /api/laputa/events/changelog
GET    /api/laputa/events/errors
```

Tauri command names should mirror behavior, not physical storage:

```text
laputa_create_proposal
laputa_list_proposals
laputa_update_proposal
laputa_apply_proposal
laputa_read_snapshot
laputa_read_section
laputa_list_changelog
laputa_read_changelog_detail
laputa_rollback_changelog
```

### 5.7 MemoryProvider Boundary

Keep `agent-diva-core::memory::MemoryProvider` as the prompt/runtime adapter
boundary. The existing trait is the correct direction: core owns the domain
request/response types and runtime code does not import storage-specific types.

Laputa should not plug into `MemoryProvider` until the proposal/apply/changelog
loop is proven. The implementation order is:

1. File store and proposal API.
2. Apply/changelog/audit/rollback.
3. E2E proof loop.
4. Read-only snapshot/section adapter.
5. Prompt rendering through `MemoryProvider`.

`MemoryProvider::sync_turn` must not become a hidden direct write path for
authority changes. It may collect evidence or create proposals after governance
exists.

## 6. AutoDream Architecture

### 6.1 Crate Role

`agent-diva-autodream` owns the reflection worker. It is a proposal and report
producer, not an authority writer.

It must implement:

- manual runs first;
- lock and checkpoint;
- input collection;
- restricted prompt execution;
- proposal/report output;
- run records;
- daily/weekly markdown report generation.

### 6.2 Storage Layout

```text
.agent-diva/
  autodream/
    lock
    checkpoint
    events.jsonl
    runs/
      {run_id}/
        autodream_run.json
        proposals.json
        stdout.md
        error.json
    reports/
      daily/{YYYY-MM-DD}.md
      weekly/{YYYY-Www}.md
```

AutoDream never writes `.laputa/state.json`, legacy `MEMORY.md`, report-system
monthly paths, or Mentle databases.

### 6.3 Run Contract

Required operations:

```text
trigger manual run
get run status
cancel run
list run outputs
list run records
write daily report
write weekly report
emit proposals
```

Manager routes:

```text
POST   /api/autodream/runs
GET    /api/autodream/runs/{id}
POST   /api/autodream/runs/{id}/cancel
GET    /api/autodream/runs/{id}/outputs
GET    /api/autodream/runs
```

Tauri commands:

```text
trigger_autodream
get_autodream_run_status
cancel_autodream_run
list_autodream_run_outputs
list_autodream_run_records
```

### 6.4 Restricted Execution

AutoDream prompt execution uses a restricted profile:

- read recent sessions;
- read Laputa snapshot/sections through API only;
- write `.agent-diva/autodream/*` outputs;
- create proposals through Laputa proposal API;
- no arbitrary shell;
- no Mentle writes;
- no `.laputa` direct write;
- no report-system monthly writes.

Manual run stabilization precedes rhythm/heartbeat automation. Auto/session
threshold mode stays off by default until the authority loop is stable.

## 7. Report System Architecture

Report System remains a consumer/display layer:

- daily reports are read from `.agent-diva/autodream/reports/daily/*`;
- weekly reports are read from `.agent-diva/autodream/reports/weekly/*`;
- monthly reports remain Report-owned;
- session search is evidence discovery, not authority;
- report solidification creates proposals, then approved proposals apply through
  Laputa.

Current GUI actions in `NotebookView.vue` invoke:

```text
solidify_report_as_sop
solidify_report_as_skill
update_memory_from_report
```

These semantics must migrate from direct durable writes to proposal creation:

| Current action | New behavior |
|---|---|
| `solidify_report_as_sop` | create `SopCreate` proposal |
| `solidify_report_as_skill` | create `SopCreate` proposal with skill sub-target |
| `update_memory_from_report` | create `MemoryPatch`, `LearningNote`, `IdentityPatch`, or `RelationshipUpdate` proposal |

The UI may keep the visible command labels, but backend semantics must say
"proposal created" until the proposal is approved and applied.

## 8. SelfImprove And Evolution UI

Add `Evolution` as the governance UI workspace. It is separate from Settings and
from the existing Notebook display surface.

### 8.1 Evolution Workspace

Evolution owns:

- Inbox: proposals pending review or needing attention;
- proposal detail: evidence, diff, target section, risk, run source;
- actions: approve, reject, edit, apply, rollback where eligible;
- Audit: changelog list/detail and rollback flow;
- diagnostics: AutoDream runs and needs_attention errors.

### 8.2 Notebook Migration

Notebook actions become "Create Proposal" flows. Notebook does not apply durable
memory, SOP, or skill changes directly.

Notebook still owns:

- report list and detail display;
- daily/weekly/monthly tabs;
- search/filter for reports;
- entry point into proposal creation.

### 8.3 Settings

Settings is policy configuration, not a review surface.

Allowed settings:

- evolution enabled;
- manual/rhythm trigger policy;
- `user_must_approve_all_writes`;
- rollback window display;
- AutoDream run limits.

Settings must not contain auto-merge affordances for v1 durable subject changes.

### 8.4 Chat

Chat can trigger AutoDream manually and show a run card. It can deep-link to
Evolution when proposals are created. Chat does not apply proposals.

## 9. Mentle Boundary

Mentle is explicitly excluded from v1 evolution governance.

Allowed:

- existing feature-gated Mentle tool settings;
- existing tool runtime behavior;
- existing compatibility tests.

Forbidden in EVO-DIVA v1:

- per-session writes into Mentle as part of evolution;
- report sync into Mentle;
- AutoDream indexing into Mentle;
- Mentle as a proposal evidence pipeline;
- Mentle participation in apply, audit, rollback, or report solidification;
- default context injection from Mentle for EVO-DIVA;
- new EVO-DIVA dependencies on `mentle_active()`.

The current `agent-diva-agent/src/mentle_runtime.rs` uses `memory/palace.db` and
feature-gated runtime assembly. Preserve it as compatibility behavior, but do
not route EVO-DIVA governance through it.

## 10. Context Compaction Boundary

Context Compaction is session-local prompt survival:

- it may summarize current-session context under context pressure;
- it may be cited as secondary evidence;
- it must not write Laputa, Mentle, MEMORY, identity, relationship, preferences,
  commitments, reports, SOPs, or skills;
- it must not be the sole evidence for a durable proposal.

## 11. Session Storage Prerequisite

`agent-diva-core/src/session/manager.rs::SessionManager::save` currently writes
the final JSONL file directly. Before AutoDream and report search depend on
session history, change it to atomic temp-write plus rename.

Required behavior:

```text
write {session}.jsonl.tmp-{pid}-{timestamp}
fsync file where supported
rename temp -> {session}.jsonl
fsync sessions dir where supported
on load: ignore stale temp files, report corrupt final files explicitly
```

This is a prerequisite for:

- AutoDream input collection;
- session search;
- report generation;
- proposal evidence references to session content.

## 12. Implementation Sequence

### Phase 0 - Contracts

1. Add `agent-diva-core::evolution` domain types.
2. Add direct-write grep tests to define the ban.
3. Add `SessionManager::save` atomic-write behavior.

### Phase 1 - Laputa Authority Loop

1. Create `agent-diva-laputa`.
2. Implement `.laputa` state skeleton and migrations.
3. Implement proposals and state transitions.
4. Implement apply, changelog, audit, and rollback.
5. Expose manager/Tauri APIs.
6. Prove the E2E loop:
   `create proposal -> approve/apply -> changelog -> read snapshot -> rollback -> audit`.

### Phase 2 - AutoDream Manual Runs

1. Create `agent-diva-autodream`.
2. Implement lock/checkpoint/run records.
3. Implement restricted input collection.
4. Implement manual run and proposal output.
5. Implement daily/weekly markdown report writes.
6. Expose manager/Tauri APIs.

### Phase 3 - Report And Notebook Migration

1. Read daily/weekly paths from AutoDream.
2. Keep monthly path isolation.
3. Replace solidification direct-write semantics with proposal creation.
4. Add tests proving no report flow writes authority directly.

### Phase 4 - Evolution UI

1. Add Evolution workspace.
2. Add Inbox review flow.
3. Add Audit changelog and rollback flow.
4. Add Chat manual trigger card and deep links.
5. Keep Settings as policy-only.

### Phase 5 - Prompt Consumption

1. Add Laputa read adapter for `MemoryProvider`.
2. Render authority sections using Always/Relevant/Archive rules.
3. Ensure prompt payload distinguishes authority content from untrusted evidence.

## 13. Test Plan

### 13.1 Domain Tests

- proposal state transitions;
- invalid transition errors;
- proposal type routing;
- section validation;
- schema version handling;
- rollback window behavior;
- compaction-only evidence rejection.

### 13.2 Laputa File-System Tests

- `.laputa` atomic writes;
- staging recovery;
- changelog and audit creation;
- rollback recovery;
- lock timeout and stale lock recovery;
- direct-write grep guards outside `agent-diva-laputa`.

### 13.3 E2E Proof Loop

Required named test:

```text
create proposal
  -> approve/apply
  -> changelog exists
  -> audit exists
  -> read snapshot includes change
  -> rollback
  -> snapshot restored
  -> audit shows apply and rollback
```

### 13.4 AutoDream Tests

- lock/checkpoint behavior;
- stale lock recovery;
- manual run success;
- manual run failure;
- cancellation;
- restricted tool profile;
- daily report path writes;
- weekly report path writes;
- proposal output schema.

### 13.5 Report And Notebook Tests

- daily path consumption;
- weekly path consumption;
- monthly path isolation;
- SOP proposal creation from report;
- Skill proposal creation from report;
- memory proposal creation from report;
- no direct authority writes.

### 13.6 GUI Tests

- Evolution Inbox review flow;
- Notebook proposal creation flow;
- Settings has no auto-merge affordance;
- Chat manual trigger card;
- Audit rollback flow.

### 13.7 Regression Tests

- existing Mentle tool settings remain available;
- existing feature-gated Mentle tests still pass;
- no EVO-DIVA flow writes to Mentle;
- Context Compaction does not write authority paths.

## 14. Open Decisions

1. Whether to mirror accepted sibling decision documents into
   `agent-diva-pro/docs/dev/genericagent/` before implementation. Architecture
   recommendation: mirror them to remove sibling-path fragility.
2. Whether `state.json` remains one file after v1 or splits into per-section
   files. Architecture recommendation: keep v1 file-first and revisit only
   after apply/rollback performance data exists.
3. Whether low-risk auto-merge is ever allowed. Architecture recommendation:
   explicitly out of v1; future policy PRD only after trust metrics exist.

## 15. Acceptance Gate

Implementation is ready only when all of these are true:

- shared domain types exist in `agent-diva-core`;
- `agent-diva-laputa` owns the only authority write implementation;
- manager/Tauri commands expose the required Laputa and AutoDream operations;
- `SessionManager::save` is atomic;
- Notebook solidification creates proposals;
- Evolution Inbox can apply and rollback through Laputa;
- daily/weekly reports are consumed from AutoDream paths;
- monthly reports remain Report-owned;
- Mentle remains compatibility-only;
- direct-write tests prove no EVO-DIVA path bypasses Laputa.
