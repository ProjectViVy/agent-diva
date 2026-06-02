# Agent-Diva Pro Self-Evolution UI Research

> Status: research and handoff document.
> Target branch: `agent-diva-pro`.
> Scope: define the complete UI surface for the next self-evolution workstream so another engineer can implement it without reopening the main product decisions.

## 1. Goal

This document specializes the earlier DivaGeneric UI direction for the **self-evolution** workstream.

The goal is not to create a generic "AI dashboard". The goal is to expose the minimum UI surfaces needed for:

- AutoDream and rhythm-driven reflection;
- candidate review and approval;
- evidence inspection;
- archive and re-wakeup flows;
- changelog and rollback visibility;
- safe configuration of self-evolution behavior.

The UI must respect the accepted architecture:

```text
GenericAgent runs the runtime
Laputa owns continuity files and rhythm artifacts
AutoDream produces reviewable proposals
Context compaction remains separate
```

This means the UI must help the user **review and govern** evolution, not silently automate it.

## 2. Existing Baseline

Current references already define part of the direction:

- `docs/dev/genericagent/newedge/ui-design.md`
- `docs/dev/genericagent/autonomous-evolution-simplified-architecture-decision.md`
- `docs/dev/genericagent/candidate-evidence-journal-audit-design.md`
- `docs/dev/genericagent/context-compaction-vs-autonomous-evolution-decision.md`

Current GUI facts:

- `NormalMode.vue` already owns the main sidebar and view switching.
- `ChatView.vue` already owns the conversation surface.
- `SettingsView.vue` already owns the settings subpages.
- current manager routes already expose `chat`, `sessions`, `config`, `tools`, `skills`, `cron`, and `providers`.
- there is no current `journal`, `cards`, `proposal inbox`, `autodream run`, `memory changelog`, or `rollback` API surface.

So the implementation does **not** start from zero, but it also does **not** have the required self-evolution UI primitives yet.

## 3. Product Decision

The self-evolution UI should be split into four product surfaces:

1. `Chat` for decisions and execution.
2. `Journal` for archive, review, and re-wakeup.
3. `Inbox` for candidate approval, apply, and rollback workflows.
4. `Settings` for self-evolution policy and runtime switches.

This is the right split because each surface answers a different user question:

- `Chat`: what needs my attention right now?
- `Journal`: what did Diva conclude over time?
- `Inbox`: what durable changes are pending or recently applied?
- `Settings`: what is the allowed behavior of the system?

## 4. What Needs UI

The next self-evolution workstream needs UI in these areas.

### 4.1 Chat-native structured cards

This is the highest-priority area.

Self-evolution requires user decisions. Those decisions should appear in the main conversation thread, not in a hidden admin panel.

Required card types:

- `PlanCard`
- `OptionCard`
- `ReviewCard`
- `JournalRefCard`
- `EvolutionProposalCard`
- `EvidencePeekCard`

New self-evolution-specific card purposes:

- `EvolutionProposalCard`: approve, reject, edit, apply, or defer memory and identity proposals.
- `EvidencePeekCard`: inspect the minimal evidence bundle for a proposal without leaving chat.

### 4.2 Journal view

Journal remains the archive and re-wakeup surface.

For self-evolution, Journal must show:

- daily reports;
- weekly reports;
- monthly reports;
- autodream summaries;
- linked proposal IDs;
- linked session IDs;
- related plan IDs;
- evidence references;
- amendment history when a later rerun produces a follow-up entry.

### 4.3 Proposal inbox

This is the main new page implied by the accepted architecture.

The user needs a dedicated place to review long-memory-adjacent changes without searching through chat history.

The inbox must show:

- pending candidates;
- recently approved candidates;
- recently applied candidates;
- reverted candidates;
- rejected candidates for audit.

### 4.4 Self-evolution settings

The accepted architecture has multiple runtime switches and policy boundaries. They need an explicit UI.

Required controls:

- Generic self-evolution enabled or disabled.
- AutoDream enabled or disabled.
- trigger mode: manual, daily, weekly, monthly.
- learning mode: off, candidate only, confirm before write, policy auto low risk.
- review policy for sensitive sections.
- Mentle exposure policy for daily chat.
- Mentle exposure policy for background reflection workers.

### 4.5 Run and health visibility

The user needs lightweight operational visibility for AutoDream and rhythm.

Required visibility:

- last run time;
- trigger source;
- run status;
- proposals produced;
- reports produced;
- last error;
- lock or in-progress state.

### 4.6 Memory changelog and rollback visibility

The architecture explicitly requires changelog and rollback information for durable memory mutation.

The UI must expose:

- recent `MEMORY.md` changes;
- affected section;
- candidate source;
- previous content preview;
- rollback availability;
- rollback action history.

## 5. Recommended Information Architecture

Recommended sidebar order for `agent-diva-pro`:

```text
Chat
Journal
Inbox
Pet
Settings
Console
Neuro
Cron
```

Why this order:

- `Chat` remains the primary action center.
- `Journal` is the first archive surface.
- `Inbox` is the first governance surface.
- `Settings` stays separate from active review work.

`Inbox` should be a top-level item, not buried under settings, because reviewing proposed memory changes is operational work, not static configuration.

## 6. Surface-by-Surface Design

### 6.1 Chat

Chat remains the main action center.

New responsibilities:

- display evolution proposals inline;
- request confirmation for sensitive memory writes;
- show rhythm wakeups;
- reopen journal context;
- show minimal evidence inline;
- move the user into plan execution when follow-up work is required.

Chat should **not**:

- become a full changelog browser;
- become a full journal reader;
- directly edit authority files;
- become a second settings screen.

### 6.2 Journal

Journal is the read-focused archive surface.

Recommended layout:

- top tabs: `日报`, `周报`, `月报`, `AutoDream`.
- left list: newest-first entries.
- main detail pane: markdown summary and linked records.
- right compact rail or footer actions: continue in chat, rerun autodream, open linked proposals, inspect evidence.

Recommended metadata block in detail view:

- period;
- status;
- related sessions;
- related plans;
- linked proposals;
- evidence count;
- generated by run ID;
- amended by later run IDs.

### 6.3 Inbox

Inbox is the proposal governance center.

Recommended tabs:

- `待处理`
- `已批准`
- `已应用`
- `已回滚`
- `已拒绝`

Recommended list columns:

- candidate type;
- target section or target file;
- risk level;
- confidence;
- created time;
- source run;
- current state.

Recommended detail panel:

- summary;
- rationale;
- proposed patch;
- previous content;
- evidence list;
- review requirement reason;
- audit trail;
- actions.

Primary actions:

- `批准`
- `拒绝`
- `编辑后批准`
- `应用`
- `回滚`
- `查看证据`
- `回到对话`

### 6.4 Settings

Recommended new settings subpage:

```text
Settings
  -> Self Evolution
```

Recommended sections inside that page:

- runtime
- triggers
- review policy
- memory safety
- Mentle integration
- diagnostics

This should stay operational and dense, not decorative.

## 7. Card Model

The existing card direction should be extended, not replaced.

Recommended shared model:

```text
UiCard {
  id
  kind
  status
  title
  summary
  body_markdown
  actions
  links
  badges
  evidence_preview
  created_at
  updated_at
}
```

Recommended `kind` values for this workstream:

```text
plan
option
review
journal_ref
evolution_proposal
evidence_peek
```

Recommended `status` additions:

```text
pending_review
approved
rejected
edited
applied
reverted
needs_attention
run_failed
```

## 8. Core User Flows

### 8.1 Rhythm wakeup flow

```text
background rhythm decides work is due
  -> Chat shows ReviewCard
  -> user chooses start / archive only / later / cancel
  -> autodream run completes
  -> report is written to Journal
  -> proposals are written to Inbox
```

### 8.2 Candidate approval flow

```text
AutoDream produces candidate
  -> Inbox receives candidate
  -> Chat may also show EvolutionProposalCard if attention is needed now
  -> user reviews rationale and evidence
  -> user approves / rejects / edits
  -> runtime applies via approved lifecycle only
  -> changelog and audit are written
```

### 8.3 Journal re-wakeup flow

```text
user opens journal entry
  -> clicks re-wakeup autodream
  -> chat receives JournalRefCard
  -> new review or proposal cycle starts
  -> old journal entry stays immutable
  -> new follow-up entry is created
```

### 8.4 Rollback flow

```text
user opens applied candidate
  -> opens changelog details
  -> sees previous content and rollback eligibility
  -> confirms rollback
  -> runtime restores previous content
  -> audit log records revert
  -> UI marks original candidate reverted
```

### 8.5 Continue from proposal flow

```text
proposal suggests unfinished project or open thread
  -> user selects continue in chat
  -> chat opens JournalRefCard or EvolutionProposalCard context
  -> Diva proposes plan or next action
  -> work continues in normal chat flow
```

## 9. Required Backend/UI Contract

The current manager API is not enough for this workstream.

Recommended new HTTP routes:

```text
GET  /api/journal?kind=daily|weekly|monthly|autodream
GET  /api/journal/:id
POST /api/journal/:id/wakeup

GET  /api/inbox/candidates?state=...
GET  /api/inbox/candidates/:id
POST /api/inbox/candidates/:id/action

GET  /api/evolution/runs
POST /api/evolution/run
GET  /api/evolution/health

GET  /api/memory/changelog
GET  /api/memory/changelog/:id
POST /api/memory/changelog/:id/rollback

GET  /api/cards/active
POST /api/cards/:id/action
```

Recommended SSE or event additions:

```text
AgentEvent::CardPresented
AgentEvent::CardUpdated
AgentEvent::JournalEntryCreated
AgentEvent::CandidateCreated
AgentEvent::CandidateStateChanged
AgentEvent::AutoDreamRunStarted
AgentEvent::AutoDreamRunCompleted
AgentEvent::AutoDreamRunFailed
AgentEvent::MemoryChangelogWritten
AgentEvent::MemoryRollbackCompleted
```

## 10. Storage Mapping the UI Must Respect

The UI must not invent a new storage layer.

It must map to these existing design decisions:

```text
.agent-diva/
  autodream/
  audit/
  plans/
  compact/

.laputa/
  rhythm/
  inbox/

memory/
  MEMORY.md
  HISTORY.md
  changelog.jsonl
```

UI implications:

- plan state comes from `.agent-diva/plans/`
- journal state comes from `.laputa/rhythm/`
- proposal inbox comes from `.laputa/inbox/`
- durable memory history comes from `memory/changelog.jsonl`
- audit views can reference `.agent-diva/audit/events.jsonl`

The UI must never write directly to those files. It must call manager or runtime APIs.

## 11. Priority and Sequencing

Recommended implementation order for `agent-diva-pro`:

### P0

- extend chat message model to support cards;
- render `ReviewCard`, `JournalRefCard`, `EvolutionProposalCard`;
- add `Journal` sidebar item and read-only `JournalView`;
- add `Inbox` sidebar item and read-only candidate list/detail;
- add self-evolution settings page skeleton.

### P1

- wire candidate actions;
- wire journal wakeup;
- wire run health and manual run;
- add changelog list and applied-candidate detail;
- add evidence drawer.

### P2

- add rollback UI;
- add richer audit trail;
- add batch actions for low-risk approvals;
- add amendment and follow-up visualization between journal entries.

This order matches the architecture. Review surfaces arrive before automatic durability.

## 12. Non-goals

The first implementation should not do these things:

- create a giant all-in-one self-evolution dashboard;
- let the GUI directly edit `MEMORY.md`;
- turn Journal into a second action center;
- merge context compaction UI with AutoDream UI;
- expose full Mentle maintenance tools in normal chat by default;
- auto-apply identity or relationship changes without review UI;
- build a visual knowledge graph before the inbox and changelog exist.

## 13. Concrete Handoff for the Next Engineer

If this work is handed to another engineer, their implementation brief should be:

```text
Build self-evolution UI for agent-diva-pro by adding:
1. chat-native card rendering
2. journal archive page
3. proposal inbox page
4. self-evolution settings page
5. minimal run health surface
6. changelog and rollback visibility
```

They should start in these local files:

- `agent-diva-gui/src/components/NormalMode.vue`
- `agent-diva-gui/src/components/ChatView.vue`
- `agent-diva-gui/src/components/SettingsView.vue`
- new `agent-diva-gui/src/components/JournalView.vue`
- new `agent-diva-gui/src/components/InboxView.vue`
- new `agent-diva-gui/src/components/cards/*`
- new `agent-diva-gui/src/api/journal.ts`
- new `agent-diva-gui/src/api/inbox.ts`
- new `agent-diva-gui/src/api/evolution.ts`
- `agent-diva-manager/src/server.rs`
- `agent-diva-manager/src/handlers.rs`

## 14. Final Recommendation

The right name for this UI initiative is not "self-evolution page".

The right product framing is:

```text
Self-Evolution UI =
chat-native review
+ archive and re-wakeup
+ proposal governance
+ safe runtime controls
```

That framing matches the accepted architecture and gives the next engineer a concrete build target instead of an abstract research theme.
