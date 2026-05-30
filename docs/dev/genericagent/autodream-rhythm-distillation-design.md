# AutoDream Rhythm Distillation Design for Agent-Diva

> Status: design specification document. This document defines the AutoDream rhythm distillation architecture for Agent-Diva. It does not implement code.
> Depends on: `compression-taxonomy-decision.md`, `compression-research.md`, `autodream-migration-research.md`, `newedge/architecture.md`, `newedge/ui-design.md`.

## 1. Executive Summary

AutoDream rhythm distillation is Agent-Diva's mechanism for periodically reviewing accumulated session evidence, history, and memory to produce **auditable long-term memory candidates, Journal entries, and action suggestions** — without blocking the live conversation and without directly mutating `MEMORY.md`.

**Recommended MVP (P0):**

- Manual trigger via manager command or `MemoryProvider` API.
- Session-end opportunistic eligibility check (non-blocking, records only).
- Lock + checkpoint file at `.agent-diva/autodream/`.
- AutoDream worker reads recent sessions + `HISTORY.md` + `MEMORY.md` rendered context.
- Produces structured `autodream_run.json` with candidates and journal drafts.
- Does **not** write `MEMORY.md` directly.
- Records evidence refs and `review_required` flag.

## 2. Problem Statement

Agent-Diva currently has `consolidation.rs` — a session-segment summarizer that fires when unconsolidated messages exceed 100. It reads truncated messages (500 chars each), sends them to the LLM, and directly rewrites `MEMORY.md` and `HISTORY.md` via `MemoryProvider::sync_turn()`. There are no structured outputs, no evidence references, no candidate review, no re-run capability, and no rhythm awareness.

This design solves the **long-term subject continuity** problem, not the single-session context window exhaustion problem. The three mechanisms are distinct:

| Mechanism | Purpose | Trigger | Output | Authority |
|---|---|---|---|---|
| Context compaction | Session-local context survival | Token/message budget pressure | Session-local compact summary | Non-authoritative, lossy |
| Memory consolidation | Extract durable facts from current session | Message count threshold | Direct MEMORY.md + HISTORY.md rewrite | Authoritative but unreviewed |
| **AutoDream rhythm distillation** | Long-term continuity refinement | Rhythm gates + manual | Candidates + Journal + evidence refs | Candidate-only until reviewed |

AutoDream does not replace consolidation. It operates on a longer time horizon, across multiple sessions, and produces auditable structured outputs instead of direct memory rewrites.

## 3. Terminology and Boundaries

### 3.1 Context Compaction

- Session-local, temporary.
- Triggered by context window pressure.
- Summarizes older turns to keep the current session alive.
- Never touches `MEMORY.md`.
- Owned by: `agent-diva-agent/src/agent_loop/`, `agent-diva-agent/src/context.rs`.

### 3.2 Memory Consolidation

- Current-session scope.
- Triggered by message count threshold (currently 100).
- Reads existing memory, merges new facts, writes directly to `MEMORY.md` + `HISTORY.md`.
- No structured output, no evidence chain, no review gate.
- Owned by: `agent-diva-agent/src/consolidation.rs`.
- Future direction: should evolve to produce Source Capsules instead of direct writes (see `compression-research.md`).

### 3.3 AutoDream Rhythm Distillation

- Cross-session, rhythmic scope.
- Triggered by time gates, session count gates, or manual command.
- Reads sessions, history, memory, capsules, Mentle recall, learning state.
- Produces `memory_patch_candidate`, `journal_entry`, `learning_candidate`, `evidence_refs`, `confidence`, `review_required`.
- Does not directly write `MEMORY.md` unless policy and confidence allow.
- Owned by: future AutoDream worker module.

## 4. Reference: Claude Code AutoDream

### 4.1 Gate Architecture

Claude Code's AutoDream uses a three-gate model (cheapest first):

```text
1. Time gate:     hours since lastConsolidatedAt >= minHours (default 24)
2. Session gate:  transcript count with mtime > lastConsolidatedAt >= minSessions (default 5)
3. Lock gate:     .consolidate-lock file (PID + mtime = lastConsolidatedAt)
```

Additional throttling:

- Scan throttle: 10-minute cooldown between session-gate scans.
- Enabled gate: requires `autoDreamEnabled` setting or `tengu_onyx_plover` feature flag.
- Current session excluded from session count.

### 4.2 Automatic vs Manual Trigger

| Dimension | Automatic (`executeAutoDream`) | Manual (`/dream`) |
|---|---|---|
| Entry point | `stopHooks.ts` → fire-and-forget | User command in main loop |
| Execution | Forked subagent, `querySource = "auto_dream"` | Runs in main REPL |
| Tool permissions | Restricted | Full |
| Blocking | Never blocks main reply | Blocks until complete |
| Timestamp update | On successful lock acquisition | Optimistic (at prompt build time) |
| Transcript | `skipTranscript = true` | Full transcript |

### 4.3 Lock Mechanism

```text
File: memory/.consolidate-lock
Content: holder PID
Mtime: lastConsolidatedAt timestamp

Acquire: write PID → set mtime = now → return prior mtime
Release: successful run completes (mtime stays as checkpoint)
Rollback: failed fork → restore prior mtime, clear PID
Stale: PID not alive AND mtime older than 60 minutes → reclaimable
```

### 4.4 Worker Execution

Claude Code uses `runForkedAgent()`:

- Spawns a subagent with `querySource = "auto_dream"`.
- `skipTranscript = true` — no session transcript written.
- Uses cache-safe params from main session.
- `onMessage` watcher folds progress into DreamTask UI.
- Supports cancel/kill from UI.

### 4.5 Consolidation Prompt (Four Phases)

```text
Phase 1 — Orient:     ls memory dir, read entrypoint, skim topic files
Phase 2 — Gather:     grep transcripts for narrow terms, check for drift
Phase 3 — Consolidate: merge into topic files, convert relative dates, fix contradictions
Phase 4 — Prune/Index: update entrypoint under line/size limits
```

### 4.6 Relationship to compact

Claude Code's compact mechanism (context compaction) and AutoDream are independent:

- Compact handles context window survival within a session.
- AutoDream handles cross-session memory refinement.
- They share `createAutoMemCanUseTool()` for permission model but not execution path.
- AutoDream reads raw session transcripts, not compact summaries.

## 5. Current Agent-Diva Memory/Rhythm Path

### 5.1 Current Flow

```text
Turn end
  -> save_turn(session, messages)
  -> if should_consolidate(session, 100)
     -> consolidate(session, provider, model, workspace, memory_provider)
        1. Slice unconsolidated messages (keep last 50 as overlap)
        2. Truncate each message to 500 chars
        3. Read existing memory via MemoryProvider::system_prompt_block()
        4. Send to LLM with save_memory tool schema
        5. Write via MemoryProvider::sync_turn() → MEMORY.md + HISTORY.md
        6. Advance last_consolidated pointer
     -> on error: log error, do not advance pointer
  -> sessions.save(session)
```

### 5.2 Existing `MemoryProvider` Lifecycle Hooks

```rust
trait MemoryProvider {
    fn system_prompt_block(&SystemPromptRequest) -> Result<SystemPromptResponse>;
    async fn prefetch(PrefetchRequest) -> Result<PrefetchResponse>;
    async fn sync_turn(SyncTurnRequest) -> Result<SyncTurnResponse>;
    async fn on_session_end(SessionEndRequest) -> Result<SessionEndResponse>;
}
```

- `system_prompt_block()`: startup prompt injection. Synchronous, no async I/O.
- `prefetch()`: live-turn recall. Read-only, no durable writes.
- `sync_turn()`: post-turn persistence. Writes to MEMORY.md + HISTORY.md.
- `on_session_end()`: session shutdown hook. Idempotent.

### 5.3 `HybridMemoryProvider` (with Mentle)

Wraps `MemoryManager` + `MentleToolkit`:

- Reads: Markdown memory (authoritative fallback) + Mentle palace.
- Writes: Markdown first, then Mentle secondary write.
- Mentle failure does not block Markdown persistence.
- Cached palace snapshot for startup prompt rendering.

### 5.4 Heartbeat Service

`agent-diva-core/src/heartbeat/service.rs`:

- Periodic heartbeat with configurable interval.
- `trigger_now()` for manual on-demand execution.
- Event-driven architecture with `HeartbeatEvent` types.

This is the closest existing primitive to AutoDream's trigger model.

### 5.5 Gap Analysis

| Dimension | Current State | Required for AutoDream |
|---|---|---|
| Trigger | Message count only | Time gate, session count gate, manual, startup catch-up |
| Scope | Single session segment | Cross-session, daily/weekly/monthly |
| Output | Direct MEMORY.md rewrite | Structured candidates, journal, evidence refs |
| Evidence | None (500-char truncation) | Session IDs, turn indices, excerpt hashes, capsule refs |
| Review | None | Confidence scoring, review_required flag, user confirmation |
| Concurrency | No lock | Lock file, checkpoint, stale recovery |
| Rhythm | None | Daily/weekly/monthly cadence awareness |
| Re-run | Not possible | Capsule-based re-consumption |

## 6. Proposed Rhythm Trigger Model

### 6.1 Gate Architecture

Agent-Diva's AutoDream should use a layered gate model adapted from Claude Code but extended for Diva's rhythm semantics:

```text
Gate 1 — Enabled:
  config.autodream.enabled == true

Gate 2 — Time (P1+):
  hours_since(last_success_at) >= config.autodream.min_hours
  default: 24h

Gate 3 — Session Count (P1+):
  sessions_touched_since(last_success_at) >= config.autodream.min_sessions
  default: 5 sessions
  current session excluded

Gate 4 — Lock:
  .agent-diva/autodream/autodream.lock not held by live process
  stale threshold: 60 minutes

Gate 5 — Cooldown:
  time_since(last_gate_check) >= scan_cooldown
  default: 10 minutes (prevents per-turn stat overhead)
```

### 6.2 Trigger Types

| Trigger | Gate Behavior | Blocking | Output Visibility |
|---|---|---|---|
| **Session-end opportunistic** | Check all gates; if open, spawn background worker | Never blocks main reply | Background task status in event bus |
| **Manual command** | Bypasses time/session gates; still checks lock | Blocks until complete or user cancels | Returns readable report to chat |
| **Startup catch-up (P1)** | Check if last_success_at is stale relative to startup time | Non-blocking background | Event bus notification |
| **Time gate (P1)** | Standard gate check during session-end | Non-blocking | Background task |
| **Session count gate (P1)** | Standard gate check during session-end | Non-blocking | Background task |
| **GUI/Manager trigger** | Equivalent to manual; passes trigger context | Returns report | Chat or Journal surface |

### 6.3 MVP Trigger Strategy

```text
MVP (P0):
  - Manual trigger only (manager command or explicit API call)
  - Session-end check: records eligibility to event bus, does not spawn worker
  - Lock file prevents concurrent runs
  - Checkpoint records last_success_at and last_session_id

P1:
  - Add time gate (24h default)
  - Add session count gate (5 sessions default)
  - Add startup catch-up (if stale, spawn background run)

P2:
  - Daily/weekly/monthly rhythm scheduling
  - Mood/project/topic-sensitive rhythm
  - Cross-instance coordination for multi-channel deployments
```

### 6.4 Checkpoint State File

```json
// .agent-diva/autodream/checkpoint.json
{
  "schema_version": 1,
  "last_success_at": "2026-05-30T14:00:00Z",
  "last_session_id": "slack:C12345",
  "last_run_id": "run-20260530-abc123",
  "last_trigger_kind": "session_end",
  "total_runs": 42,
  "total_candidates_produced": 156,
  "total_candidates_accepted": 89
}
```

### 6.5 Lock File

```text
File: .agent-diva/autodream/autodream.lock
Content: { "pid": 12345, "started_at": "2026-05-30T14:00:00Z", "trigger_kind": "manual" }
Stale: PID not alive AND started_at older than 60 minutes
```

## 7. Worker Execution Model

### 7.1 Execution Strategy Comparison

| Strategy | Pros | Cons | Recommendation |
|---|---|---|---|
| Direct in agent loop `await` | Simple | Blocks main reply, violates non-blocking requirement | **Never** |
| `tokio::spawn` background task | Non-blocking, shares runtime | No tool isolation, crash risk to main process | MVP fallback |
| Manager/service worker | Process isolation, GUI-visible | Requires manager API changes | P1 target |
| Forked subagent | Tool isolation, progress events, cancel | Requires subagent infrastructure | P1+ target |
| External command/job queue | Full isolation | Over-engineered for current scale | Not recommended |

### 7.2 Recommended Progression

**MVP (P0): Background Task Abstraction**

```text
AutoDreamWorker::spawn(config, evidence_gatherer, llm_provider, memory_context)
  -> tokio::spawn(async move {
       acquire_lock()?;
       gather_evidence().await?;
       run_llm_distillation().await?;
       write_outputs().await?;
       update_checkpoint()?;
       release_lock()?;
       emit_completion_event();
     })
  -> return AutoDreamHandle { cancel_token, status_receiver }
```

Key constraints:

- Never `await` inside `process_inbound_message_inner()`.
- Session save must complete before AutoDream reads it.
- AutoDream failure must not affect main reply.
- Checkpoint updates only after successful output write.
- `cancel_token` allows external cancellation.

**P1: Restricted Subagent Runner**

```text
AutoDreamSubagentRunner
  -> spawn controlled subagent via SubagentManager
  -> restricted tool registry (read-only filesystem, no shell, no web)
  -> progress events via MessageBus
  -> cancel support via AbortHandle
  -> output written to .agent-diva/autodream/runs/<run_id>/
```

**P2: Manager/Service Job Queue**

```text
AutoDreamJobQueue (in agent-diva-manager)
  -> GUI-visible job status
  -> retry policy
  -> concurrent run prevention
  -> resource budget (LLM calls, time limit)
```

### 7.3 Failure Isolation

```text
AutoDream failure modes and responses:

1. LLM timeout/error → log error, rollback lock, retry next gate open
2. LLM returns unparseable output → log error, save raw output for debugging, rollback lock
3. Output write failure → log error, rollback lock, retry next gate open
4. Lock acquisition failure → skip silently (another run in progress)
5. Evidence gather failure → log warning, proceed with partial evidence, mark run as degraded
6. Cancel requested → save partial progress, rollback lock, emit cancel event
7. Process killed → stale lock recovery on next attempt (60-minute threshold)
```

## 8. Input Sources

### 8.1 Input Priority Order

| Priority | Source | Access Method | Notes |
|---|---|---|---|
| 1 | Raw session messages | `SessionManager::get()` | Primary evidence. Not truncated summaries. |
| 2 | `HISTORY.md` | Direct file read via `MemoryManager::load_history()` | Chronological event stream |
| 3 | `MEMORY.md` current version | `MemoryProvider::system_prompt_block()` | Current state to reason against |
| 4 | Source Capsules (when available) | `CompressionStore` (future) | Pre-distilled evidence from compression layer |
| 5 | Mentle recall (if enabled) | `MentleToolkit` query | Dense factual recall for specific topics |
| 6 | LearningCandidate state | `.laputa/inbox/learning-candidates.jsonl` | Pending candidates to avoid duplication |
| 7 | Journal history | `.laputa/rhythm/` directory scan | Prior rhythm outputs for continuity |
| 8 | User explicit feedback | `.laputa/inbox/decisions.jsonl` | Past accept/reject decisions |

### 8.2 Critical Constraint

AutoDream **must not** use context compaction summaries as sole evidence. Context compaction produces lossy working summaries for session-local survival. They can supplement raw sessions but cannot replace them.

When Source Capsules are available (from the compression layer described in `compression-research.md`), they should be preferred over raw session scanning for efficiency, but the worker must retain the ability to drill into raw sessions when capsule summaries are insufficient.

### 8.3 Evidence Window

The worker should gather evidence from:

- Sessions touched since `last_success_at` (from checkpoint).
- Most recent N history entries (configurable, default 30 days).
- Current MEMORY.md full content.
- Any new Source Capsules created since `last_success_at`.
- Pending learning candidates (for deduplication).

## 9. Output Schema

### 9.1 AutoDream Run Output

```json
{
  "schema_version": 1,
  "run_id": "run-20260530-abc123",
  "run_at": "2026-05-30T14:30:00Z",
  "trigger": {
    "kind": "manual|session_end|time_gate|session_count_gate|startup_catchup",
    "reason": "User requested via /dream command"
  },
  "source_window": {
    "session_ids": ["slack:C12345", "telegram:U67890"],
    "history_range": "2026-05-28 to 2026-05-30",
    "capsule_ids": ["cap-20260529-x1y2z3"],
    "memory_snapshot_hash": "sha256:abc123"
  },
  "memory_patch_candidates": [
    {
      "candidate_id": "mpc-001",
      "target": "MEMORY.md",
      "operation": "append|replace_section|revise",
      "section": "identity|relationship|project|preference|rule|fact",
      "proposed_text": "...",
      "rationale": "User confirmed preference for dark mode in 3 separate sessions",
      "evidence_refs": [
        {
          "source": "session",
          "session_id": "slack:C12345",
          "turn_index": 12,
          "excerpt_hash": "sha256:abc123",
          "excerpt_preview": "I prefer dark mode for all my tools"
        }
      ],
      "confidence": 0.85,
      "review_required": true,
      "auto_merge_eligible": false
    }
  ],
  "journal_entries": [
    {
      "entry_id": "je-001",
      "title": "Provider routing fix completed",
      "body": "Fixed DeepSeek model ID prefix issue. Raw model ID for native endpoints, provider/model for gateways.",
      "entry_type": "daily|weekly|monthly",
      "source_capsule_ids": ["cap-20260529-x1y2z3"],
      "evidence_refs": [
        {
          "source": "session",
          "session_id": "slack:C12345",
          "turn_range": [10, 25],
          "excerpt_hash": "sha256:def456"
        }
      ]
    }
  ],
  "learning_candidates": [
    {
      "candidate_id": "lc-001",
      "content": "Always verify model ID format when modifying provider routing",
      "suggested_layer": "L3SopOrSkill",
      "evidence_refs": [],
      "confidence": 0.9,
      "review_required": true
    }
  ],
  "risks": [
    "Mentle full profile tool isolation not yet implemented for background workers"
  ],
  "open_questions": [
    "Should weekly reports include cross-channel session aggregation?"
  ],
  "next_actions": [
    "Review memory_patch_candidate mpc-001",
    "Accept learning_candidate lc-001 into SOP"
  ],
  "run_metadata": {
    "duration_ms": 45000,
    "llm_calls": 3,
    "tokens_used": 12000,
    "evidence_items_scanned": 156,
    "degraded": false,
    "degradation_reason": null
  }
}
```

### 9.2 Output Location

```text
.agent-diva/autodream/
  checkpoint.json          — last success state
  autodream.lock           — concurrency lock
  runs/
    run-20260530-abc123/
      autodream_run.json   — full structured output
      raw_llm_output.json  — raw LLM response for debugging
      evidence_manifest.json — list of evidence items consumed
```

### 9.3 Output Lifecycle

1. Worker writes `autodream_run.json` to run directory.
2. Worker emits event on `MessageBus` with run summary.
3. Chat surface can render `ReviewCard` with candidates.
4. User reviews candidates (accept/reject/edit).
5. Accepted candidates are applied through `MemoryProvider` or Generic Core.
6. Accepted journal entries are written to `.laputa/rhythm/`.
7. Checkpoint updated only after successful output write.

## 10. MEMORY.md Merge Policy

### 10.1 Default Policy: No Direct Write

AutoDream **does not** directly modify `MEMORY.md` by default. It generates `memory_patch_candidate` entries in the run output. These candidates require explicit action before becoming part of long-term memory.

### 10.2 Merge Eligibility Tiers

| Tier | Section Types | Confidence Threshold | Review Required | Auto-merge Eligible |
|---|---|---|---|---|
| **Critical** | identity, relationship, rule | Any | Always | Never |
| **Sensitive** | preference, expectation | < 0.95 | Always | Never |
| **Standard** | project state, fact | >= 0.9 | Optional | With explicit user policy |
| **Low-risk** | formatting fix, date correction, dedup | >= 0.8 | Optional | With explicit user policy |

### 10.3 Merge Process

```text
Candidate arrives (from AutoDream run)
  -> Tier classification (by section type + confidence)
  -> if Critical or Sensitive:
       write to .laputa/inbox/learning-candidates.jsonl
       emit OptionCard in chat
       wait for user decision
  -> if Standard and user policy allows auto-merge:
       apply via MemoryProvider::sync_turn()
       record in changelog with evidence_refs
       emit notification in chat
  -> if Low-risk and user policy allows auto-merge:
       apply silently
       record in changelog
```

### 10.4 Changelog Requirements

Every merge into `MEMORY.md` must record:

- `run_id` of the AutoDream run that produced the candidate.
- `candidate_id` of the specific patch candidate.
- `merged_at` timestamp.
- `merge_method`: `user_confirmed` | `policy_auto` | `manual_review`.
- `evidence_refs` carried from the candidate.
- Previous content (for rollback).

Changelog location: `.agent-diva/autodream/changelog.jsonl`.

### 10.5 Rollback

Every merge must be reversible. The changelog entry contains the previous content. A rollback operation reads the changelog and restores the prior state.

## 11. Journal and Evidence Chain

### 11.1 Journal as AutoDream Output

Journal entries are a natural output of AutoDream rhythm distillation:

- **Not** long-term memory. Journal is a structured archive.
- **References** source capsule IDs and evidence refs.
- **Immutable** once written. Corrections create follow-up or amendment records.
- **Queryable** by date, topic, evidence source, or linked candidates.

### 11.2 Journal Entry Schema

```json
{
  "entry_id": "je-20260530-001",
  "created_at": "2026-05-30T14:30:00Z",
  "created_by": "autodream_worker",
  "entry_type": "daily",
  "title": "Provider routing fix + memory architecture research",
  "body": "## Completed\n- Fixed DeepSeek model ID prefix bug\n- Designed compression taxonomy\n\n## In Progress\n- AutoDream rhythm distillation design\n\n## Open\n- Mentle tool isolation for background workers",
  "source_run_id": "run-20260530-abc123",
  "source_capsule_ids": ["cap-20260529-x1y2z3"],
  "evidence_refs": [
    {
      "source": "session",
      "session_id": "slack:C12345",
      "turn_range": [10, 25],
      "excerpt_hash": "sha256:def456"
    }
  ],
  "linked_candidates": ["mpc-001", "lc-001"],
  "linked_plans": ["plan-provider-fix"],
  "linked_sessions": ["slack:C12345", "telegram:U67890"]
}
```

### 11.3 JournalRefCard in Chat

When a user re-opens a journal entry from the Journal UI:

1. GUI switches to `ChatView`.
2. Chat receives a `JournalRefCard` identifying the source entry.
3. Diva summarizes the reopened context.
4. User decides next action in chat.
5. Original journal entry remains immutable; new work creates new entries.

### 11.4 Evidence Chain Traceability

Every AutoDream output is traceable to its sources:

```text
Journal entry
  <- AutoDream run (run_id)
     <- Source window (session_ids, history_range, capsule_ids)
        <- Raw sessions (with turn indices)
        <- Source Capsules (with their own evidence_refs)
        <- HISTORY.md entries
  <- Linked candidates (candidate_ids)
     <- Their evidence_refs
```

This ensures that any claim in `MEMORY.md` can be traced back through the candidate, the AutoDream run, to the original session evidence.

## 12. Integration Points

### 12.1 Files and Modules to Modify

| Module | Change | Phase |
|---|---|---|
| `agent-diva-core/src/memory/provider.rs` | No changes needed; AutoDream operates outside `MemoryProvider` | — |
| `agent-diva-core/src/config/` | Add `autodream` config section | P0 |
| `agent-diva-agent/src/agent_loop/loop_turn.rs` | Add session-end eligibility check (non-blocking) | P0 |
| `agent-diva-agent/src/` (new: `autodream/`) | AutoDream worker, gate logic, prompt builder | P0 |
| `agent-diva-core/src/` (new: `autodream/`) | Checkpoint, lock, run output types | P0 |
| `agent-diva-manager/` | Manual trigger command endpoint | P0 |
| `agent-diva-agent/src/subagent.rs` | Extend for AutoDream subagent runner | P1 |
| `agent-diva-core/src/bus/` | Add `AutoDreamEvent` types | P1 |
| `agent-diva-gui/` | ReviewCard rendering, Journal tab | P2 |

### 12.2 New Types in `agent-diva-core`

```rust
// agent-diva-core/src/autodream/types.rs

pub struct AutoDreamConfig {
    pub enabled: bool,
    pub min_hours: u32,         // default: 24
    pub min_sessions: u32,      // default: 5
    pub scan_cooldown_secs: u64, // default: 600
    pub stale_lock_secs: u64,   // default: 3600
    pub max_evidence_items: usize, // default: 200
}

pub struct AutoDreamCheckpoint {
    pub last_success_at: DateTime<Utc>,
    pub last_session_id: String,
    pub last_run_id: String,
    pub last_trigger_kind: TriggerKind,
    pub total_runs: u64,
    pub total_candidates_produced: u64,
    pub total_candidates_accepted: u64,
}

pub struct AutoDreamRunOutput {
    pub schema_version: u32,
    pub run_id: String,
    pub run_at: DateTime<Utc>,
    pub trigger: TriggerInfo,
    pub source_window: SourceWindow,
    pub memory_patch_candidates: Vec<MemoryPatchCandidate>,
    pub journal_entries: Vec<JournalEntryDraft>,
    pub learning_candidates: Vec<LearningCandidateDraft>,
    pub risks: Vec<String>,
    pub open_questions: Vec<String>,
    pub next_actions: Vec<String>,
    pub run_metadata: RunMetadata,
}
```

### 12.3 Integration with Existing `on_session_end()`

The `MemoryProvider::on_session_end()` hook is the natural place for the session-end eligibility check. However, AutoDream itself should **not** run inside `on_session_end()`. Instead:

```text
on_session_end()
  -> existing cleanup (idempotent)
  -> AutoDream eligibility check (non-blocking)
     -> if gates open: emit event on bus
     -> if gates closed: record check timestamp, skip
  -> return immediately

AutoDreamWorker (separate from MemoryProvider)
  -> subscribes to AutoDreamEvent on bus
  -> spawns background task when event received
```

This preserves the `MemoryProvider` contract: `on_session_end()` remains idempotent and non-blocking.

## 13. Failure Modes

### 13.1 Failure Matrix

| Failure | Detection | Response | Recovery |
|---|---|---|---|
| Lock stale (process killed) | PID not alive AND mtime > 60min | Reclaim lock | Next attempt proceeds normally |
| Lock held by live process | PID alive | Skip silently | Wait for next gate open |
| LLM timeout | tokio timeout | Rollback lock | Retry next gate open |
| LLM unparseable output | JSON parse failure | Save raw output, log error, rollback lock | Manual inspection; retry next gate |
| LLM empty output | No candidates produced | Log info, update checkpoint, release lock | Normal — no useful signal this cycle |
| Evidence file missing | IO error during read | Log warning, skip item, proceed with partial | File may have been archived |
| Checkpoint corruption | Invalid JSON | Reset checkpoint, log warning | Start fresh from next run |
| Output write failure | IO error | Rollback lock, log error | Retry next gate open |
| Duplicate run | Lock held | Skip | Lock prevents this |
| Session save race | Worker reads session mid-save | Read session metadata timestamp, skip if too recent | 5-second grace period |
| Candidate conflict | Two candidates propose different values for same section | Present both to user for disambiguation | User resolves in OptionCard |

### 13.2 Graceful Degradation

When the worker encounters partial failures:

- Mark the run as `degraded: true` in `run_metadata`.
- Include `degradation_reason` explaining what was missing.
- Still produce candidates from available evidence.
- Log which evidence sources were unavailable.
- Do not treat degraded runs as failures for checkpoint purposes (checkpoint still advances).

## 14. Testing Strategy

### 14.1 Unit Tests

| Test Area | What to Assert |
|---|---|
| Gate logic | Time gate opens after min_hours; session gate opens after min_sessions; cooldown prevents rapid re-check |
| Lock acquisition | Success when unlocked; failure when held; stale reclaim when PID dead + old mtime |
| Lock rollback | Mtime restored to prior value on failure |
| Checkpoint read/write | Round-trip serialization; missing file creates default; corrupt file resets |
| Output schema validation | All required fields present; enum values valid; evidence_refs non-empty where required |
| Merge policy classification | Correct tier assignment for each section type + confidence combination |

### 14.2 Integration Tests

| Test Area | What to Assert |
|---|---|
| End-to-end manual trigger | Lock acquired → evidence gathered → output written → checkpoint updated → lock released |
| Non-blocking guarantee | Main reply completes before AutoDream worker finishes |
| Session-end eligibility | Correct event emitted when gates open; no event when gates closed |
| Failure isolation | AutoDream error does not propagate to main reply |
| Concurrent run prevention | Second trigger skipped while first is running |

### 14.3 Schema Tests

| Test Area | What to Assert |
|---|---|
| Run output JSON schema | Valid against documented schema |
| Candidate schema | All required fields; evidence_refs format |
| Journal entry schema | Entry type, evidence refs, linked candidates |
| Checkpoint schema | Version field; migration path for future versions |

## 15. MVP / P1 / P2 Plan

### 15.1 P0 / MVP

**Scope:** Manual trigger + session-end eligibility check + structured output.

**Deliverables:**

1. `agent-diva-core/src/autodream/` — checkpoint, lock, config, run output types.
2. `agent-diva-agent/src/autodream/` — worker, gate logic, prompt builder, evidence gatherer.
3. `agent-diva-manager` — manual trigger command endpoint.
4. `agent-diva-agent/src/agent_loop/loop_turn.rs` — session-end eligibility check (non-blocking event emission).
5. Unit tests for gate logic, lock, checkpoint, schema validation.
6. Integration test for end-to-end manual trigger.

**Constraints:**

- Worker uses `tokio::spawn` background task (not forked subagent yet).
- Tool permissions: read-only filesystem access only.
- No automatic merge into `MEMORY.md`.
- All candidates require manual review.
- Output stored as `autodream_run.json`.

### 15.2 P1

**Scope:** Rhythm gates + restricted subagent + candidate/journal integration.

**Deliverables:**

1. Time gate (24h default) + session count gate (5 sessions default).
2. Startup catch-up (if stale at startup, spawn background run).
3. Restricted subagent runner via `SubagentManager`.
4. `AutoDreamEvent` on `MessageBus` for GUI integration.
5. Candidate inbox integration: accepted candidates written to `.laputa/inbox/learning-candidates.jsonl`.
6. Journal draft integration: journal entries written to `.laputa/rhythm/daily/`.
7. `ReviewCard` rendering in chat for candidate review.

### 15.3 P2

**Scope:** Richer rhythm + Mentle + auto-merge + GUI.

**Deliverables:**

1. Daily/weekly/monthly rhythm scheduling.
2. Mentle recall integration for dense factual evidence.
3. Automatic low-risk merge policy (configurable by user).
4. GUI Journal tab with entry browsing, re-wakeup, and evidence drill-down.
5. `JournalRefCard` rendering in chat for journal re-open.
6. Changelog and rollback infrastructure.
7. Cross-channel session aggregation for multi-channel deployments.

## 16. Open Questions

These require product or architecture confirmation before implementation:

1. **AutoDream prompt structure:** Should the LLM prompt follow Claude Code's four-phase model (Orient → Gather → Consolidate → Prune), or should it use a different structure adapted for Diva's candidate-inbox paradigm?

2. **Evidence budget:** What is the maximum evidence window (in sessions, days, or tokens) for a single AutoDream run? This affects LLM cost and run duration.

3. **Mentle tool exposure in background worker:** The `agent-diva-generic` design says AutoDream can use "full 30+ capabilities." Should the MVP restrict this, and if so, to what subset?

4. **Source Capsule dependency:** Should AutoDream MVP wait for the compression layer (Source Capsules), or should it start with raw session scanning and upgrade when capsules are available?

5. **Multi-channel aggregation:** When Agent-Diva runs on Slack + Telegram + QQ simultaneously, should AutoDream produce per-channel or cross-channel rhythm outputs?

6. **User confirmation UX:** For P1, should candidate review happen inline in chat (OptionCard) or in a dedicated review surface? The UI design doc suggests both are possible.

7. **Journal retention policy:** How long do daily/weekly/monthly journal entries remain in active storage before archival?

8. **`sync_turn()` evolution:** Should the existing `consolidation.rs` eventually be replaced by AutoDream's compression + distillation pipeline, or should they coexist indefinitely as separate mechanisms?

---

## Appendix A: Mapping to Claude Code AutoDream

| Claude Code Concept | Agent-Diva Equivalent | Notes |
|---|---|---|
| `autoDream.ts` → `initAutoDream()` | `AutoDreamWorker::init()` | Closure-scoped state |
| `executeAutoDream()` | `AutoDreamWorker::check_and_spawn()` | Session-end opportunistic |
| `config.ts` → `isAutoDreamEnabled()` | `AutoDreamConfig::enabled` | Config-driven |
| `consolidationLock.ts` | `agent-diva-core/src/autodream/lock.rs` | File-based lock |
| `consolidationPrompt.ts` | `agent-diva-agent/src/autodream/prompt.rs` | Diva-specific prompt |
| `/dream` skill | Manager command or explicit API | Manual entry point |
| `DreamTask` UI | `ReviewCard` + Journal tab | Diva card-based UX |
| `runForkedAgent()` | `SubagentManager` (P1) / `tokio::spawn` (P0) | Worker execution |
| `backgroundHousekeeping.init()` | `AgentLoop` initialization or manager startup | Bootstrap |
| `stopHooks.ts` → fire-and-forget | `loop_turn.rs` → event emission | Non-blocking |

## Appendix B: Relationship to Existing Architecture Docs

| Document | How This Design Extends It |
|---|---|
| `compression-taxonomy-decision.md` | Implements the "AutoDream rhythm distillation" track specifically |
| `compression-research.md` | AutoDream consumes Source Capsules as evidence; compression layer is a prerequisite for efficient evidence gathering |
| `autodream-migration-research.md` | This design is the concrete architecture that migration research recommended |
| `newedge/architecture.md` | AutoDream operates on the offline path; its outputs feed L4 rhythm archives and Inbox candidates |
| `newedge/ui-design.md` | AutoDream outputs map to `ReviewCard`, `JournalRefCard`, and Journal tab entries |
