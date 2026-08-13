---
stepsCompleted: [1, 2, 3, 4]
lastStep: 4
status: 'complete'
revision: v1.0.0-evo-diva
inputDocuments:
  - _bmad-output/planning-artifacts/prds/prd-evo-diva-governance-2026-06-12/prd.md
  - docs/architecture/evo-diva-architecture-2026-06-12.md
  - docs/prds/prd-laputa-2026-06-12/prd.md
  - docs/prds/prd-autodream-2026-06-12/prd.md
  - docs/prds/prd-selfinprove-2026-06-12/prd.md
  - docs/prd-report-system/prd.md
  - _bmad-output/planning-artifacts/ux-designs/ux-agent-diva-2026-06-12/DESIGN.md
  - _bmad-output/planning-artifacts/ux-designs/ux-agent-diva-2026-06-12/EXPERIENCE.md
workflowType: 'epics-stories'
project_name: 'agent-diva-pro'
user_name: 'mastwet'
date: '2026-06-13'
---

# agent-diva-pro - EVO-DIVA Epic Breakdown

## Overview

This document provides the complete epic and story breakdown for EVO-DIVA governance. It decomposes the governance PRD, the current EVO-DIVA architecture, the Laputa, AutoDream, SelfImprove, and Report System PRDs, and the Evolution UX design into implementable stories.

The governing authority spine is:

`EvidenceRef -> EvolutionProposal -> user review -> Laputa apply -> changelog/audit -> rollback -> prompt/report consumption`

Mentle remains existing tool integration only in v1. Context Compaction remains session-local and cannot become durable authority.

## Requirements Inventory

### Functional Requirements

| Requirement | Description |
| --- | --- |
| GOV-FR101-GOV-FR103 | Use a unified `EvolutionProposal` envelope, route proposal types, and enforce proposal state transitions. |
| GOV-FR201-GOV-FR203 | Centralize durable writes through Laputa, expose changelog/rollback, stable read APIs, and event/polling boundaries. |
| GOV-FR301-GOV-FR303 | Keep AutoDream thin, manual-first, and able to hand off daily/weekly reports. |
| GOV-FR401-GOV-FR404 | Provide Evolution Inbox, Journal/Audit visibility, Chat manual trigger, and policy/diagnostics settings. |
| GOV-FR501-GOV-FR503 | Keep Report System as consumer/display, make solidification create proposals, and treat session search as evidence only. |
| GOV-FR601-GOV-FR603 | Preserve existing Mentle tool integration while excluding Mentle from v1 evolution governance. |
| GOV-FR701-GOV-FR702 | Keep Context Compaction session-local and secondary evidence only. |
| L-FR101-L-FR106 | Implement Laputa write boundary, type routing, governance gates, locking, merge conflict handling, and Mentle read-only recall boundaries. |
| L-FR201-L-FR205 | Implement snapshot reads, section reads, incremental snapshots, changelog reads, and explicit TBD section status. |
| L-FR301-L-FR303 | Emit proposal, changelog, error, and needs-attention events with polling fallback. |
| L-FR401-L-FR403 | Provide paginated changelog, changelog details, and rollback. |
| L-FR501-L-FR506 | Support the 14-section schema, legacy mapping, migration, template deprecation, bootstrap handling, and schema versioning. |
| L-FR507-L-FR509 | Connect the required hooks, prove the E2E loop, and restrict direct user edits to allowed sections. |
| AD-FR1-AD-FR4 | Trigger AutoDream manually, keep auto mode off by default, enforce lock files, and maintain checkpoints. |
| AD-FR5-AD-FR9 | Collect inputs, run restricted forked execution, apply the four-stage prompt, write structured artifacts, and append event logs. |
| AD-FR10-AD-FR14 | Display outputs, collect user decisions through governance, generate daily/weekly reports, and report failures. |
| SI-FR101-SI-FR109 | Provide proposal inbox list/detail, review actions, batch actions, read/unread, rollback, Chat deep-link, search/filter, and keyboard shortcuts. |
| SI-FR201-SI-FR205 | Convert report solidification actions to Laputa proposal/apply flows with rollback and batch handling. |
| SI-FR301-SI-FR305 | Configure AutoDream policy, expose manual trigger, show diagnostics, enforce user approval for writes, and poll as fallback. |
| SI-FR401-SI-FR403 | Proxy changelog list/detail/rollback into SelfImprove/Evolution surfaces. |
| SI-FR501-SI-FR603 | Respect Laputa section routing, migration/deprecation, and Mentle governance boundaries. |
| RS-FR1-RS-FR5 | Display daily/weekly AutoDream reports, generate monthly reports in Report-owned storage, trigger generation, and render reports in Notebook. |
| RS-FR6-RS-FR10 | Convert SOP/Skill/Memory solidification to proposals and keep agent session search as evidence. |

### NonFunctional Requirements

| Requirement | Description |
| --- | --- |
| GOV-FR801-GOV-FR805 | Ensure auditability, reversibility, failure isolation, path stability, and prompt safety. |
| L-FR701-L-FR706 | Meet Laputa performance targets, audit completeness, rollback completeness, concurrency safety, typed errors, and metrics. |
| SI-FR701-SI-FR706 | Meet Evolution UI performance, audit, rollback, concurrency, error, and observability requirements. |
| RS-NFR | Preserve report performance, reliability, security, privacy, and LLM/cost guardrails. |
| UX-NFR | Provide keyboard access, visible focus, non-color-only status, destructive confirmations, stable responsive layouts, and non-overlapping text. |

### Additional Requirements

- Add `agent-diva-laputa` as the in-repo authority crate with file-first storage under `.laputa/`.
- Add `agent-diva-autodream` as the manual-first reflection crate with storage under `.agent-diva/autodream/`.
- Add shared governance domain types to `agent-diva-core`.
- Keep `agent-diva-core::memory::MemoryProvider` as the runtime adapter boundary.
- Expose canonical Rust APIs through manager HTTP endpoints and Tauri commands for GUI use.
- Replace direct Notebook solidification command semantics with proposal creation.
- Make `SessionManager::save` atomic before AutoDream/report search depend on session history.
- Add direct-write prevention tests for authority files.

### UX Design Requirements

- Add a top-level `Evolution` workspace with Inbox, Runs, Audit, and Policy tabs.
- Default to Inbox with proposal filters, search, counts, batch actions, list/detail layout, and evidence drawer.
- Proposal detail must show title, status, risk, source, target, summary, proposed change, diff, evidence, affected modules, safety checks, and governance actions.
- Runs must show trigger, time, duration, status, inputs, outputs, errors, and proposal counts.
- Audit must show timestamp, actor, source proposal, target, change type, summary, and rollback availability.
- Notebook actions must be renamed to Create SOP Proposal, Create Skill Proposal, and Create Memory Proposal.
- Settings must configure policy only and include the required v1 review copy.
- Chat must support manual AutoDream trigger cards and compact Evolution proposal cards.
- Keyboard shortcuts `/`, `J`, `K`, `Enter`, `A`, `E`, `R`, `D`, and `Esc` must work outside text inputs.

### FR Coverage Map

| Requirement Group | Epic | Coverage |
| --- | --- | --- |
| GOV-FR101-GOV-FR203, L-FR101-L-FR506, L-FR507-L-FR509 | Epic 1 | Core proposal, authority, apply, changelog, snapshot, event, rollback, schema, and session-save behavior. |
| GOV-FR401-GOV-FR404, SI-FR101-SI-FR109, SI-FR301-SI-FR305, SI-FR401-SI-FR403, UX requirements | Epic 2 | Evolution workspace, review UX, policy, diagnostics, audit, rollback, Chat links, and accessibility. |
| GOV-FR301-GOV-FR303, AD-FR1-AD-FR14 | Epic 3 | Manual AutoDream runs, restricted execution, events, proposal outputs, and daily/weekly reports. |
| GOV-FR501-GOV-FR503, SI-FR201-SI-FR205, RS-FR1-RS-FR10 | Epic 4 | Report/Notebook consumption, monthly isolation, proposal-based solidification, and session evidence search. |
| GOV-FR601-GOV-FR702, SI-FR501-SI-FR603 | Epic 5 | Runtime consumption, Mentle exclusion, MemoryProvider boundary, and Context Compaction limits. |
| GOV-FR801-GOV-FR805, L-FR701-L-FR706, SI-FR701-SI-FR706, RS-NFR | Epic 6 | Migration, recovery, typed errors, test proof loop, performance, audit, rollback, and observability. |

## Epic List

### Epic 1: Governed Authority Spine
Users can safely turn durable evolution changes into reviewable proposals and apply them through Laputa with changelog, audit, and rollback.
**FRs covered:** GOV-FR101-GOV-FR203, L-FR101-L-FR506, L-FR507-L-FR509.

### Epic 2: Evolution Review Console
Users can review, approve, reject, edit, defer, apply, and rollback proposals from a dedicated Evolution workspace.
**FRs covered:** GOV-FR401-GOV-FR404, SI-FR101-SI-FR109, SI-FR301-SI-FR305, SI-FR401-SI-FR403, UX requirements.

### Epic 3: Manual AutoDream Reflection Runs
Users can manually trigger reflection runs, inspect outputs, and receive proposals plus daily/weekly reports without silent authority changes.
**FRs covered:** GOV-FR301-GOV-FR303, AD-FR1-AD-FR14.

### Epic 4: Report and Notebook Proposal Solidification
Users can consume reports and convert findings into SOP, skill, or memory proposals without bypassing governance.
**FRs covered:** GOV-FR501-GOV-FR503, SI-FR201-SI-FR205, RS-FR1-RS-FR10.

### Epic 5: Runtime Consumption and Boundary Enforcement
Diva consumes only applied Laputa authority while Mentle remains existing tool integration and Context Compaction stays session-local.
**FRs covered:** GOV-FR601-GOV-FR702, SI-FR501-SI-FR603.

### Epic 6: Migration, Recovery, and Proof Loop
Maintainers can migrate legacy authority safely, recover from failures, and prove the governance loop end to end.
**FRs covered:** GOV-FR801-GOV-FR805, L-FR701-L-FR706, SI-FR701-SI-FR706, RS-NFR.

## Epic 1: Governed Authority Spine

**Epic Goal:** Users can safely turn durable evolution changes into reviewable proposals and apply them through Laputa with changelog, audit, and rollback.

### Story 1.1: Define Governance Domain Types

As a developer,
I want shared governance domain types in `agent-diva-core`,
So that Laputa, AutoDream, Report System, and Evolution UI use one contract.

**Acceptance Criteria:**

**Given** the workspace builds `agent-diva-core`
**When** the governance module is added
**Then** it exposes `EvidenceRef`, `EvidenceSource`, `EvolutionProposal`, `ProposalType`, `ProposalState`, `LaputaSectionName`, `ChangelogRecord`, `AuditEvent`, `RollbackRequest`, and `AutoDreamRunRecord`
**And** each type supports serialization, deserialization, clone/debug behavior, and stable schema-friendly field names
**And** proposal types route to valid Laputa section names or return a typed unknown-type error.

### Story 1.2: Create Laputa File-First Authority Storage

As a maintainer,
I want a dedicated `agent-diva-laputa` crate with file-first storage,
So that durable authority is owned by one implementation boundary.

**Acceptance Criteria:**

**Given** the workspace Cargo manifest is loaded
**When** `agent-diva-laputa` is added
**Then** it builds as a workspace crate and owns `.laputa/state.json`, proposals, changelog, audit, rollback staging, migrations, locks, and legacy storage paths
**And** all writes use temp-file plus rename behavior
**And** file locking works on Windows, Linux, and macOS with timeout and stale recovery handling.

### Story 1.3: Implement Proposal CRUD and State Transitions

As a user,
I want proposals to be created, listed, updated, and reviewed consistently,
So that every durable evolution change has an explicit lifecycle.

**Acceptance Criteria:**

**Given** a valid proposal create request with evidence refs
**When** Laputa persists the proposal
**Then** it appears in list responses with status, type, target, source, risk, timestamps, and evidence count
**And** valid transitions follow the architecture state machine: pending_review, approved, rejected, edited, applied, reverted, superseded, needs_attention, and run_failed
**And** invalid state transitions return typed errors without mutating persisted proposal state.

### Story 1.4: Apply Proposal Through Governance Gates

As a reviewer,
I want approved proposals to apply only through Laputa,
So that subject, memory, SOP, skill, identity, relationship, commitment, preference, and policy writes are auditable and reversible.

**Acceptance Criteria:**

**Given** an approved proposal targets a valid Laputa section
**When** the reviewer applies it
**Then** Laputa validates the section, stages rollback data, writes the new authority state, appends changelog, appends audit, and marks the proposal applied
**And** any failure during apply rolls back the staged write atomically
**And** unknown proposal types, unauthorized targets, schema mismatches, and unresolved conflicts return typed errors.

### Story 1.5: Expose Laputa Read, Changelog, Rollback, and Event APIs

As a UI or service consumer,
I want stable Laputa operations through Rust, HTTP, and Tauri boundaries,
So that Evolution UI and runtime consumers can read authority without bypassing governance.

**Acceptance Criteria:**

**Given** Laputa is initialized
**When** callers request proposal CRUD, apply, snapshot, section, changelog list/detail, rollback, or events
**Then** the Rust API, manager HTTP routes, and Tauri commands expose equivalent behavior
**And** proposal, changelog, and error events are available through SSE with polling fallback
**And** section reads for TBD sections return explicit TBD status instead of silently failing.

### Story 1.6: Make Session Saves Atomic

As AutoDream and Report System,
I want session history saves to be atomic,
So that reflection and search do not read partially written session files.

**Acceptance Criteria:**

**Given** `SessionManager::save` writes session history
**When** save is called
**Then** it writes to a temporary file and renames it into place
**And** failed writes leave the previous session file readable
**And** tests cover successful save, interrupted save, and replacement behavior.

## Epic 2: Evolution Review Console

**Epic Goal:** Users can review, approve, reject, edit, defer, apply, and rollback proposals from a dedicated Evolution workspace.

### Story 2.1: Add Evolution Workspace Shell

As a user,
I want a top-level Evolution workspace,
So that governance review is visible and separate from settings or Notebook content.

**Acceptance Criteria:**

**Given** the GUI is loaded
**When** proposals or governance events exist
**Then** the sidebar shows an Evolution workspace with a count badge
**And** the workspace contains Inbox, Runs, Audit, and Policy tabs
**And** the Inbox is the default tab
**And** responsive layout switches between split and collapsed list/detail states without overlapping text.

### Story 2.2: Build Proposal Inbox List and Filters

As a reviewer,
I want to scan, filter, search, and batch proposals,
So that review work stays fast even when AutoDream emits multiple candidates.

**Acceptance Criteria:**

**Given** proposals exist in pending or needs-attention states
**When** I open Evolution Inbox
**Then** each row shows type, status, risk, target, source, evidence count, age, and blocked reason when present
**And** filters support status, type, risk, source, target, unread, and search text
**And** batch actions support approve, reject, read/unread, and defer where the proposal state allows it
**And** keyboard shortcuts `/`, `J`, `K`, `Enter`, `A`, `E`, `R`, `D`, and `Esc` work outside text inputs
**And** empty, loading, and error states are visible without spinner-only panels.

### Story 2.3: Build Proposal Detail and Governance Actions

As a reviewer,
I want to inspect proposal details and act on them safely,
So that durable authority changes remain user-controlled.

**Acceptance Criteria:**

**Given** a proposal is selected
**When** the detail pane opens
**Then** it shows title, status, risk, source, target, timestamps, summary, proposed change, diff, evidence, affected modules, and safety checks
**And** Approve and Apply, Approve Only, Edit Proposal, Reject, Defer, and Rollback actions call the Laputa commands
**And** destructive actions require confirmation naming the target
**And** evidence opens in a drawer or inline preview with missing evidence explicitly marked
**And** high-risk proposals open with evidence visible and cannot be approved when required evidence is missing.

### Story 2.4: Build Runs, Audit, and Policy Views

As a user,
I want to see reflection runs, audit history, and governance policy,
So that I can understand what the system proposed and what was applied.

**Acceptance Criteria:**

**Given** AutoDream run records and Laputa audit events exist
**When** I open Runs or Audit
**Then** Runs shows trigger, start/end time, duration, status, inputs, outputs, errors, and proposal count
**And** Audit shows timestamp, actor, source proposal, target, change type, summary, and rollback availability
**And** Policy shows configuration only, not a review queue
**And** Policy includes the exact copy: `Durable personality, memory, SOP, skill, and policy changes require review before they are applied.`

### Story 2.5: Connect Chat and Settings to Governance

As a user,
I want Chat and Settings to initiate governance flows without hiding review,
So that manual triggers and policy edits stay understandable.

**Acceptance Criteria:**

**Given** I trigger AutoDream from Chat
**When** the run starts
**Then** Chat shows an AutoDream run card with status and a link to Evolution Runs or Inbox
**And** generated proposals render as compact Evolution proposal cards
**And** Settings exposes policy options but no auto-merge affordance
**And** apply failures preserve the proposal and show a recoverable error.

## Epic 3: Manual AutoDream Reflection Runs

**Epic Goal:** Users can manually trigger reflection runs, inspect outputs, and receive proposals plus daily/weekly reports without silent authority changes.

### Story 3.1: Implement Manual Run Lifecycle

As a user,
I want to trigger, inspect, and cancel AutoDream manually,
So that reflection is explicit before any automatic rhythm is enabled.

**Acceptance Criteria:**

**Given** no AutoDream lock exists
**When** a manual run is triggered
**Then** AutoDream creates a run record, writes a lock file, returns a run ID, and reports status through API/Tauri commands
**And** cancel requests stop the run and update status
**And** stale locks older than the configured threshold recover safely
**And** auto/session-threshold mode remains off by default.

### Story 3.2: Collect Reflection Inputs with Limits

As AutoDream,
I want to gather bounded evidence from sessions and project memory sources,
So that reflection has context without over-reading or mutating authority.

**Acceptance Criteria:**

**Given** a run starts
**When** input collection executes
**Then** it reads recent sessions, Laputa snapshots/sections through the read API, and source capsules in configured priority order
**And** missing inputs are recorded as non-fatal omissions
**And** collected inputs are summarized in the run record
**And** input collection never writes to Mentle and never directly writes Laputa authority files.

### Story 3.3: Execute Restricted Reflection Prompt

As the system,
I want AutoDream to run a restricted reflection worker,
So that proposal generation cannot directly mutate durable authority.

**Acceptance Criteria:**

**Given** inputs have been collected
**When** the worker runs
**Then** it executes the Orient, Gather, Consolidate, and Propose stages
**And** the worker uses a restricted tool profile with no arbitrary shell, no Mentle writes, and no direct authority-write tools
**And** any proposal creation uses the Laputa proposal API rather than direct `.laputa` writes
**And** timeout, cancellation, success, and failure outcomes are recorded
**And** failures produce user-visible diagnostics and do not update success checkpoint.

### Story 3.4: Emit Proposal and Run Output Artifacts

As a reviewer,
I want AutoDream outputs to become review-required proposals,
So that reflection can suggest changes without applying them.

**Acceptance Criteria:**

**Given** reflection completes successfully
**When** output artifacts are written
**Then** `.agent-diva/autodream/runs/{run_id}/autodream_run.json` contains schema version, evidence refs, confidence, output summaries, proposal candidates, and `review_required: true`
**And** emitted proposals use the shared `EvolutionProposal` contract
**And** events are appended to `.agent-diva/autodream/events.jsonl`
**And** the run record links to proposal IDs.

### Story 3.5: Generate Daily and Weekly Reports

As a user,
I want AutoDream to generate daily and weekly reports,
So that Report System can display reflection summaries without owning those outputs.

**Acceptance Criteria:**

**Given** a manual report run succeeds
**When** daily or weekly report output is requested
**Then** daily reports write to `.agent-diva/autodream/reports/daily/{date}.md`
**And** weekly reports write to `.agent-diva/autodream/reports/weekly/{week}.md`
**And** reports include evidence refs suitable for proposal creation
**And** Report System can read the files without creating authority writes.

## Epic 4: Report and Notebook Proposal Solidification

**Epic Goal:** Users can consume reports and convert findings into SOP, skill, or memory proposals without bypassing governance.

### Story 4.1: Consume AutoDream and Report-Owned Paths Correctly

As a user,
I want reports displayed from their authoritative storage locations,
So that daily, weekly, and monthly report ownership stays clear.

**Acceptance Criteria:**

**Given** reports exist
**When** Report System loads report lists
**Then** daily and weekly reports are read from `.agent-diva/autodream/reports/*`
**And** monthly reports remain under Report-owned monthly storage
**And** missing reports show placeholders with trigger actions
**And** large report files are safely truncated or lazy-loaded according to Report System requirements.

### Story 4.2: Replace Notebook Direct Solidification with Proposal Creation

As a Notebook user,
I want report actions to create reviewable proposals,
So that SOP, skill, and memory updates cannot bypass Laputa.

**Acceptance Criteria:**

**Given** a report is open in Notebook
**When** I choose Create SOP Proposal, Create Skill Proposal, or Create Memory Proposal
**Then** the UI previews target, extracted summary, evidence refs, risk, and review status
**And** submitting creates an `EvolutionProposal`
**And** no durable authority file is modified by the Notebook action
**And** the created proposal can be opened in Evolution Inbox.

### Story 4.3: Preserve Session Search as Evidence Only

As a user,
I want to search prior sessions from reports and proposals,
So that old conversations can support decisions without becoming authority.

**Acceptance Criteria:**

**Given** session search is available
**When** a report or proposal references search results
**Then** results include session ID, timestamp, snippet, and evidence metadata
**And** search results can be attached as `EvidenceRef`
**And** search results are never injected into default runtime context as durable memory
**And** corrupted session files are skipped with diagnostics.

### Story 4.4: Add Solidification Regression Coverage

As a maintainer,
I want tests proving old direct writes are gone,
So that Report and Notebook cannot regress around governance.

**Acceptance Criteria:**

**Given** legacy commands such as `solidify_report_as_sop`, `solidify_report_as_skill`, and `update_memory_from_report` exist or are migrated
**When** tests exercise those flows
**Then** they create proposals or return migration-compatible proposal responses
**And** authority writes occur only after Laputa apply
**And** direct-write grep guards fail on new EVO-DIVA direct writes outside Laputa.

## Epic 5: Runtime Consumption and Boundary Enforcement

**Epic Goal:** Diva consumes only applied Laputa authority while Mentle remains existing tool integration and Context Compaction stays session-local.

### Story 5.1: Plug Applied Laputa Reads into MemoryProvider

As Diva runtime,
I want to consume applied authority through `MemoryProvider`,
So that prompts use reviewed state without depending on Laputa internals.

**Acceptance Criteria:**

**Given** Laputa proposal, apply, changelog, and read APIs exist
**When** runtime context is built
**Then** `MemoryProvider` can read applied Laputa snapshot or section data
**And** unapplied proposals are excluded from default prompt context
**And** after Laputa is available, `ContextBuilder`, subagent identity assembly, and runtime prompt assembly do not directly read `SOUL.md`, `IDENTITY.md`, `USER.md`, `memory/MEMORY.md`, or `memory/HISTORY.md` as authority
**And** legacy authority files can be consumed only through the migration or compatibility adapter, with legacy source marked explicitly and unapplied content excluded from default prompts
**And** read failures degrade safely without creating authority writes
**And** tests cover prompt consumption of applied and unapplied changes.

### Story 5.2: Enforce Mentle Governance Exclusion

As a maintainer,
I want EVO-DIVA flows to avoid new Mentle dependencies,
So that v1 governance has one authority owner.

**Acceptance Criteria:**

**Given** existing feature-gated Mentle tool integration is enabled
**When** EVO-DIVA proposal, AutoDream, Report, Notebook, Audit, or rollback flows run
**Then** they do not write to Mentle
**And** they do not sync reports to Mentle
**And** they do not index AutoDream outputs into Mentle
**And** they do not inject Mentle recall by default into governance context.

### Story 5.3: Keep Context Compaction Session-Local

As Diva runtime,
I want Context Compaction to summarize only the active session,
So that compaction does not become hidden durable memory.

**Acceptance Criteria:**

**Given** Context Compaction creates a session summary
**When** EVO-DIVA evidence is collected
**Then** compaction output can be referenced only as secondary evidence
**And** it is not written to Laputa, Mentle, MEMORY, SOP, skill, identity, relationship, commitment, preference, or policy stores
**And** durable changes based on compaction must still create proposals.

## Epic 6: Migration, Recovery, and Proof Loop

**Epic Goal:** Maintainers can migrate legacy authority safely, recover from failures, and prove the governance loop end to end.

### Story 6.1: Implement Legacy Schema Migration

As a maintainer,
I want legacy authority material migrated into Laputa safely,
So that the new authority spine can start without losing existing state.

**Acceptance Criteria:**

**Given** legacy templates or authority files exist
**When** migration runs
**Then** legacy material is copied into `.laputa/legacy`
**And** supported material maps into the 14-section schema
**And** unsupported or TBD material is stored with explicit TBD status
**And** migration updates `.laputa/state.json` schema version without deleting legacy sources
**And** migrated legacy authority files become migration input or backup only, not runtime prompt authority.

### Story 6.2: Implement Recovery, Conflict, and Typed Error Handling

As a maintainer,
I want failures to be explicit and recoverable,
So that authority writes do not leave the system in an unknown state.

**Acceptance Criteria:**

**Given** a write, apply, migration, or rollback fails
**When** recovery logic runs
**Then** staging data is used to restore the prior safe state where possible
**And** unresolved merge conflicts mark proposals `needs_attention`
**And** typed errors include schema incompatible, unauthorized, lock timeout, conflict unresolved, rollback expired, unknown layer, and IO error cases
**And** errors emit diagnostics for UI and logs.

### Story 6.3: Add Direct-Write and Filesystem Tests

As a maintainer,
I want tests that prove authority writes are constrained,
So that future work does not bypass Laputa accidentally.

**Acceptance Criteria:**

**Given** the test suite runs
**When** direct-write guard tests inspect EVO-DIVA flows
**Then** unauthorized writes to durable authority paths fail the test
**And** direct-read guard tests fail when runtime prompt assembly reads legacy authority files directly instead of the Laputa read boundary
**And** filesystem tests cover atomic writes, lock timeout, stale lock recovery, staging recovery, and changelog/audit creation
**And** tests run on Windows-compatible paths.

### Story 6.4: Add End-to-End Governance Proof Loop

As a maintainer,
I want a named E2E test for the governance loop,
So that the core authority spine is proven before broader runtime consumption.

**Acceptance Criteria:**

**Given** a valid proposal can be created
**When** the E2E proof loop runs
**Then** it creates a proposal, approves and applies it, reads changelog, reads snapshot, rolls back changelog, and verifies audit contains both apply and rollback events
**And** the test confirms no direct writes occurred outside Laputa
**And** the test is documented as the release gate for prompt/report consumption.

### Story 6.5: Add Metrics and Release-Gate Validation

As a maintainer,
I want observability and release gates for EVO-DIVA governance,
So that failures are visible before users rely on durable evolution.

**Acceptance Criteria:**

**Given** EVO-DIVA flows run
**When** writes, write errors, rollbacks, AutoDream runs, or governance failures occur
**Then** metrics are emitted for Laputa writes, write errors, rollbacks, AutoDream runs, and failures
**And** release validation checks zero silent writes, rollback success for eligible records, manual AutoDream run stability, and review surface availability
**And** Mentle regression tests confirm no EVO-DIVA governance role in v1.
