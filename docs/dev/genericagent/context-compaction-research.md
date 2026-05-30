# Context Compaction Research for Agent-Diva

> Status: Research & design document. No code implemented.
> Date: 2026-05-31.
> Scope: Session-local context compaction design — how to keep a long session alive without polluting long-term memory.
> Out of scope: AutoDream rhythm system, Journal UI, LearningCandidate full flow, MEMORY.md long-term structure redesign, provider major refactors.

## 1. Executive Summary

Agent-Diva currently has no session-local context compaction. When a single session grows too long, the model loses context and the prompt becomes unstable. The existing `consolidation.rs` writes to MEMORY.md (long-term memory) — it is not context compaction.

**Recommended approach:**

Introduce a `ContextCompactor` that sits between session history retrieval and prompt assembly. When estimated context pressure exceeds a threshold, it compresses older turns into a structured compact summary, keeps recent raw turns intact, and injects the compact summary as a session-local boundary marker into the prompt. The compact summary never writes to MEMORY.md and never calls `MemoryProvider::sync_turn()`.

**One-line answer:**

> Agent-Diva should compress overly long sessions by summarizing older turns into a structured session-local compact checkpoint, injecting it as a boundary-marked prompt block before recent raw turns — never touching MEMORY.md or the long-term memory boundary.

## 2. Problem Statement

### 2.1 The Core Problem

Agent-Diva sessions grow unbounded. A single session can accumulate hundreds of messages with large tool results, multi-step tool call chains, and long reasoning traces. This causes:

1. **Prompt overflow**: The assembled prompt (system + memory + skills + history + current message) exceeds the provider's context window.
2. **Context instability**: Even before hitting the hard limit, large prompts cause the model to lose track of early instructions, files, and decisions.
3. **Token cost explosion**: Every turn re-sends the full history, increasing cost linearly.

### 2.2 What Context Compaction Is NOT

Context compaction is distinct from two other mechanisms already discussed:

| Mechanism | Purpose | Lifetime | Writes to MEMORY.md? |
|---|---|---|---|
| **Context compaction** (this document) | Keep the current session alive | Session-local, ephemeral | **No** |
| **Memory consolidation** (`consolidation.rs`) | Extract durable facts from session material | Long-term | Yes (via `MemoryProvider::sync_turn()`) |
| **AutoDream rhythm distillation** (future) | Periodic deep review of sessions, history, evidence | Long-term, rhythmic | Via candidate pipeline |

Context compaction is a **session survival mechanism**, not a memory extraction mechanism. Its output is a lossy working summary that helps the model continue coherent work within the same session.

## 3. Terminology

| Term | Definition |
|---|---|
| **Context compaction** | Replacing older raw session turns with a structured summary to fit within the context window. Session-local, ephemeral. |
| **Memory consolidation** | Extracting durable, reusable information from session material into long-term memory (MEMORY.md, HISTORY.md). |
| **Rhythm distillation** | AutoDream-style periodic deep review that produces journal entries, learning candidates, and memory patches. |
| **Compact summary** | The structured output of context compaction: a lossy summary of compressed turns. |
| **Compact checkpoint** | Persisted state recording what was compacted, enabling resume after process restart. |
| **Boundary marker** | A system-level annotation injected into the prompt that tells the model: "everything before this marker is a lossy summary, not raw conversation." |
| **Recent raw turns** | The N most recent messages kept verbatim (not summarized) after compaction. |
| **Context pressure** | An estimate of how close the current prompt is to the provider's context window limit. |

## 4. Reference: Claude Code Compact System

### 4.1 Architecture Overview

Claude Code implements a multi-layered compact system in `.workspace/claude-code/src/services/compact/`:

| Layer | File | Purpose |
|---|---|---|
| **Base compact** | `compact.ts` | Full conversation summarization using LLM |
| **Partial compact** | `compact.ts` | Compress only older prefix, keep recent messages |
| **Auto compact** | `autoCompact.ts` | Automatic trigger based on token threshold |
| **Micro compact** | `microCompact.ts` | Lightweight: clear old tool result content |
| **Snip compact** | `snipCompact.ts` | User-directed message removal with markers |
| **Reactive compact** | `reactiveCompact.ts` | Emergency: triggered after provider returns prompt-too-long |
| **Session memory compact** | `sessionMemoryCompact.ts` | Cross-session memory-aware compaction |

### 4.2 Trigger Mechanism

Claude Code's auto-compact trigger (`autoCompact.ts`):

```
threshold = context_window_size - MAX_OUTPUT_TOKENS (20K) - buffer (13K-20K)
should_compact = estimated_tokens >= threshold
```

Key constants:
- `AUTOCOMPACT_BUFFER_TOKENS = 13,000`
- `WARNING_THRESHOLD_BUFFER_TOKENS = 20,000`
- `TOOL_RESULT_GROWTH_ESTIMATE = 15,000` per turn
- `MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES = 3` (circuit breaker)

Token estimation uses `roughTokenCountEstimation()` with a `4/3` padding multiplier over character-based estimates.

### 4.3 Compact Prompt Structure

Claude Code uses a structured prompt (`prompt.ts`) requiring the model to output:

```xml
<analysis>
[Draft scratchpad — stripped before final summary reaches context]
</analysis>

<summary>
1. Primary Request and Intent
2. Key Technical Concepts
3. Files and Code Sections
4. Errors and fixes
5. Problem Solving
6. All user messages
7. Pending Tasks
8. Current Work
9. Optional Next Step
</summary>
```

The `<analysis>` block is a drafting scratchpad that gets stripped before the summary is injected back into context. This ensures the model thinks carefully without polluting the final output.

### 4.4 Boundary Marker

After compaction, Claude Code inserts a `SystemCompactBoundaryMessage` that marks the transition between compacted summary and recent raw messages. This boundary:

- Records whether it was auto or manual compaction
- Tracks pre-compact token count
- Carries metadata about pre-compact discovered tools
- Includes relink metadata for preserved message segments

### 4.5 Post-Compact Restore

Claude Code restores critical context after compaction:

1. **Recently accessed files**: Up to 5 files from `readFileState`, budget 50K tokens
2. **Plan file**: If a plan exists for the current session
3. **Invoked skills**: Content of skills used in the session, budget 25K tokens, 5K per skill
4. **Plan mode state**: If user is in plan mode
5. **Async agent state**: Background agents running
6. **Tool/instruction announcements**: Deferred tools, agent listings, MCP instructions

### 4.6 Reactive Compact

When the provider returns a prompt-too-long error, Claude Code catches it and attempts emergency compaction (`reactiveCompact.ts`). This is a safety net, not the primary path.

### 4.7 Micro Compact

Before each API call, Claude Code can clear old tool result content (`microCompact.ts`) to reduce context size without full summarization. This handles the common case where tool results (file reads, grep output, shell output) are the primary source of context bloat.

### 4.8 What Agent-Diva Should Borrow

| Should borrow | Should NOT borrow |
|---|---|
| Structured `<analysis>` + `<summary>` prompt pattern | GrowthBook/Statsig feature flag system |
| Boundary marker distinguishing compacted vs raw turns | Ink REPL / task store / bundled skill integration |
| Token-aware trigger with context-window ratio | `stopHooks` / React hook integration |
| Circuit breaker for consecutive failures | Prompt cache sharing / cache_editing API |
| Post-compact file/state restore | KAIROS session transcript system |
| Reactive compact as safety net | Micro compact with cache_edits API |
| Grouping messages by API-round boundaries | Partial compact (direction: from/up_to) — too complex for MVP |

## 5. Current Agent-Diva Context Path

### 5.1 Context Assembly Flow

```
user message arrives (InboundMessage)
  │
  ├─→ SessionManager.get_or_create(session_key)
  │     └─→ Session.get_history(50)  ← last 50 unconsolidated messages
  │
  ├─→ ContextBuilder.build_messages(history, current_message, channel, chat_id)
  │     ├─→ build_system_prompt()
  │     │     ├─→ identity header (IDENTITY.md)
  │     │     ├─→ static instructions (tools, workspace, time)
  │     │     ├─→ soul sections (AGENTS.md, SOUL.md, IDENTITY.md, USER.md, BOOTSTRAP.md)
  │     │     ├─→ skills (always-loaded + summary)
  │     │     └─→ MemoryProvider.system_prompt_block()  ← MEMORY.md rendered block
  │     ├─→ history → Vec<Message>  (user/assistant/tool messages)
  │     └─→ current_message → Message::user()
  │
  ├─→ [optional] prefetch intent → MemoryProvider.prefetch() → inject recall block
  │
  ├─→ Agent loop (max_iterations):
  │     ├─→ provider.chat_stream(messages, tools, model, 4096, 0.7)
  │     ├─→ if tool_calls:
  │     │     ├─→ execute each tool
  │     │     ├─→ add_assistant_message(messages, content, tool_calls)
  │     │     ├─→ add_tool_result(messages, tool_call_id, name, result)
  │     │     └─→ continue loop
  │     └─→ if no tool_calls: final response, break
  │
  ├─→ save_turn(session, messages, history_len, ...)
  │     ├─→ session.add_message(user_role, content)
  │     ├─→ for each new message in turn: session.add_full_message(...)
  │     │     └─→ tool messages truncated to 500 chars
  │     └─→ final assistant response saved
  │
  ├─→ if should_consolidate(session, 100):
  │     └─→ consolidation::consolidate() → LLM → MemoryProvider::sync_turn()
  │           └─→ writes MEMORY.md + HISTORY.md
  │
  └─→ sessions.save(session)  ← persist to disk
```

### 5.2 Key Source Files

| File | Role |
|---|---|
| `agent-diva-core/src/session/store.rs` | `Session` struct, `get_history(max_messages)`, `last_consolidated` pointer |
| `agent-diva-core/src/session/manager.rs` | `SessionManager`: load/save/list/archive sessions |
| `agent-diva-agent/src/context.rs` | `ContextBuilder`: system prompt assembly, message building |
| `agent-diva-agent/src/agent_loop/loop_turn.rs` | `process_inbound_message_inner()`: main turn logic, save_turn, consolidation trigger |
| `agent-diva-agent/src/consolidation.rs` | `consolidate()`: LLM-based summarization → MEMORY.md |
| `agent-diva-core/src/memory/provider.rs` | `MemoryProvider` trait: system_prompt_block, prefetch, sync_turn, on_session_end |
| `agent-diva-tools/src/sanitize.rs` | Tool result truncation (80K chars global, 60K file content, 500 chars in session save) |

### 5.3 Context Expansion Points

Where context grows without bound:

| Source | Growth pattern | Current mitigation |
|---|---|---|
| **Session messages** | Linear with turns. Each turn adds user + assistant + N tool call/result pairs | `get_history(50)` caps at 50 messages, but these 50 can be huge |
| **Tool results** | Spiky. File reads can be 60K chars, shell output unbounded | `truncate_tool_result()` at 80K; session save truncates at 500 chars |
| **Memory block** | Grows as MEMORY.md accumulates content | No budget; full rendered block injected |
| **Skills** | Always-loaded skills injected in full | No budget; full content in system prompt |
| **System prompt** | Grows with soul sections, Mentle guidance, tool rules | No budget; assembled from multiple files |
| **Reasoning/thinking blocks** | Can be very long for complex reasoning | Stored in session but no truncation |

## 6. Current Gaps

### 6.1 No Token-Aware Context Management

`get_history()` uses message count (50), not token budget. Fifty messages with large tool results can easily exceed 100K tokens. There is no estimation of total prompt size before calling the provider.

### 6.2 No Context Compaction Mechanism

When the session grows long, the only option is `archive_and_reset()` which loses all context. There is no way to compress older turns while preserving working state.

### 6.3 No Compact Checkpoint

If the process restarts, there is no persisted compact state. The session either has all messages or is archived — no middle ground.

### 6.4 No Boundary Marker

The model cannot distinguish between raw conversation and compacted summary. It treats everything in the history as equally authoritative and detailed.

### 6.5 No Post-Compact State Restore

After any form of context reduction, recently accessed files, active plans, and tool state are not preserved.

### 6.6 No Manual Compact Entry

Users cannot trigger context compaction manually (no `/compact` equivalent).

### 6.7 No Reactive Safety Net

If the provider returns a context-overflow error, there is no fallback — the turn fails entirely.

### 6.8 Consolidation Is Not Compaction

`consolidation.rs` writes to MEMORY.md via `MemoryProvider::sync_turn()`. It is a long-term memory extraction mechanism, not a session survival mechanism. It cannot be repurposed for context compaction because:

1. It writes to durable storage (MEMORY.md, HISTORY.md) — compaction must be ephemeral.
2. Its `last_consolidated` pointer has different semantics than a compact checkpoint.
3. Its output format (free-text `memory_update` + one-line `history_entry`) is not suitable for structured session-local working state.
4. It consumes the LLM to produce memory candidates — compaction needs the LLM to produce a working-state summary.

## 7. Proposed Architecture

### 7.1 Context Path with Compaction Insertion Point

```
user message arrives
  │
  ├─→ SessionManager.get_or_create(session_key)
  │     └─→ Session.get_history()  ← get all unconsolidated messages
  │
  ├─→ NEW: ContextBudgetMonitor.estimate_pressure(history, model)
  │     └─→ if over threshold:
  │           ├─→ ContextCompactor.compact(older_messages, provider, model)
  │           │     ├─→ LLM call with structured prompt
  │           │     ├─→ parse structured summary
  │           │     └─→ return CompactSummary
  │           ├─→ Session.set_compaction(CompactSummary)
  │           └─→ sessions.save()  ← persist compact checkpoint
  │
  ├─→ ContextBuilder.build_messages(history, current_message, ...)
  │     ├─→ build_system_prompt()  (unchanged)
  │     ├─→ if session has compaction:
  │     │     ├─→ inject boundary marker system message
  │     │     ├─→ inject compact summary as system message
  │     │     └─→ inject recent raw turns only
  │     └─→ else: inject full history
  │
  └─→ ... rest of agent loop (unchanged)
```

### 7.2 Trigger Strategy

#### P0 (MVP): Message Count + Rough Token Estimate

```rust
fn should_compact(session: &Session, model_context_window: usize) -> bool {
    let message_count = session.messages.len() - session.last_consolidated;
    let estimated_tokens = estimate_session_tokens(&session.messages[session.last_consolidated..]);

    // Primary: message count threshold
    if message_count >= COMPACTION_MESSAGE_THRESHOLD { // 80
        return true;
    }

    // Secondary: estimated token threshold
    let token_threshold = (model_context_window as f64 * 0.70) as usize;
    if estimated_tokens >= token_threshold {
        return true;
    }

    false
}
```

Token estimation: character count / 4 * (4/3), matching Claude Code's approach.

#### P1: Provider-Aware Context Window

Query the provider for its context window size. Different models have very different limits (4K for small models, 128K+ for large ones). The threshold should be a ratio of the actual window.

#### P1: Manual `/compact` Entry

Expose a `/compact` command or equivalent API that triggers compaction regardless of threshold. Useful when the user senses the model losing context.

#### P1: Reactive Compact (Safety Net)

If the provider returns a context-overflow error, attempt emergency compaction before failing the turn.

#### NOT Recommended: Tool Result Size Threshold Alone

Tool result size is a symptom, not the cause. Micro-compact (clearing old tool results) is useful but should be a separate, lighter mechanism — not the primary compaction trigger.

### 7.3 Compaction Scope

#### What to compress

All messages before the recent-N boundary:

```
[0 ─────────────────── split_point ─────────── end]
 ^^^^^^^^^^^^^^^^^^^^^^                    ^^^^^^^^^
 compress these into                      keep these
 compact summary                          as raw turns
```

**Split point calculation:**

```rust
fn calculate_split_point(messages: &[ChatMessage], keep_recent: usize) -> usize {
    let total = messages.len();
    if total <= keep_recent {
        return total; // nothing to compact
    }
    let split = total - keep_recent;

    // Adjust to avoid splitting tool_use/tool_result pairs
    // If messages[split] is a "tool" message, back up to before the tool_use
    adjust_for_tool_pairing(messages, split)
}
```

`keep_recent` default: 10 messages (configurable).

#### How to handle tool call / tool result pairs

Tool results in the compacted span are summarized as part of the conversation flow. The model should capture: "the assistant read file X and found Y" rather than preserving the raw tool result.

For recent raw turns, tool results are kept verbatim (subject to existing truncation).

#### What must be preserved across compaction

1. **Active task state**: What is being worked on right now.
2. **Unresolved questions**: Things the user asked that haven't been answered.
3. **Recent files and tools**: What files were read/modified, what tools were used.
4. **Plan state**: If a plan is active, its current step and progress.
5. **User preferences expressed in session**: Corrections, style preferences, explicit instructions.
6. **Decisions made**: Architecture decisions, naming choices, approach selections.

#### Multiple compactions (stacking)

Yes, allow multiple compactions. When a session continues to grow after the first compaction:

1. The existing compact summary + recent raw turns are the new "full history."
2. When pressure exceeds threshold again, compact the old compact summary + older raw turns into a new compact summary.
3. Only one compact summary is active at any time (the latest).
4. The compact summary field is replaced, not appended.

This means compaction is always "replace the current summary with a new, more recent one that covers more ground."

### 7.4 Compaction Prompt

Adapted from Claude Code's structured prompt, simplified for Agent-Diva:

```
你是一个会话压缩助手。请为以下对话创建一份详细摘要，保留继续工作所需的关键信息。

在给出最终摘要前，请在 <analysis> 标签中整理你的思路。

你的摘要必须包含以下部分：

1. 主要请求和意图：用户的明确请求和当前目标
2. 关键技术概念：讨论的重要技术概念、框架、架构决策
3. 涉及的文件和代码：检查、修改或创建的文件，包含关键代码片段
4. 错误和修复：遇到的错误及修复方法，特别是用户反馈
5. 已完成的工作：已解决的问题和完成的任务
6. 未完成的任务：明确待处理的任务
7. 当前工作：正在进行的具体工作，包含文件名和进度
8. 下一步建议：与最近工作直接相关的下一步（仅在任务未结束时）

请用以下格式输出：

<analysis>[你的思考过程]</analysis>

<summary>[按上述结构的摘要]</summary>

注意：这是会话内压缩，不是长期记忆。摘要应足够详细，让模型能理解上下文并继续工作。
```

### 7.5 Prompt Assembly After Compaction

```
[system prompt — unchanged]
[memory block from MemoryProvider — unchanged]

## Compacted Session Context
The following is a lossy summary of earlier turns in this same session.
It is not long-term memory and must not be treated as durable truth.
It may contain inaccuracies introduced by the summarization process.
If you need precise details from earlier conversation, ask the user.

[compact summary content]

[end of compacted context]

[recent raw turns — user/assistant/tool messages, verbatim]
[current user message]
```

The boundary marker text is deliberately cautious: it tells the model the summary is lossy and not durable truth, preventing the model from treating compact summaries as authoritative facts.

### 7.6 Post-Compact State Preservation

After compaction, the following state should be preserved or re-injected:

| State | How to preserve |
|---|---|
| Active task description | Included in compact summary's "当前工作" section |
| Recent file paths | Included in compact summary's "涉及的文件" section |
| Plan state | If plan file exists, inject as attachment (future P1) |
| User preferences in session | Included in compact summary's "主要请求" section |

For MVP, preservation relies on the compact summary quality. For P1, explicit state extraction and re-injection can be added.

## 8. Data Model

### 8.1 Compact Summary Schema

Stored in `Session.metadata` (as a JSON field):

```json
{
  "compaction": {
    "schema_version": 1,
    "compact_id": "compact-20260530-a1b2c3",
    "created_at": "2026-05-30T10:30:00Z",
    "trigger": "auto",
    "source_range": {
      "start_index": 0,
      "end_index": 120
    },
    "kept_recent_count": 10,
    "pre_compact_message_count": 130,
    "pre_compact_estimated_tokens": 85000,
    "summary": "## Compacted Session Context\n\n1. 主要请求和意图:\n  ...",
    "raw_turns_snapshot": null
  }
}
```

### 8.2 Field Definitions

| Field | Required | Description |
|---|---|---|
| `schema_version` | Yes | Schema version, currently `1`. Increment on breaking changes. |
| `compact_id` | Yes | Unique identifier: `compact-YYYYMMDD-<short_hash>` |
| `created_at` | Yes | ISO 8601 timestamp |
| `trigger` | Yes | `auto` / `manual` / `reactive` |
| `source_range.start_index` | Yes | First message index included in compaction |
| `source_range.end_index` | Yes | Last message index included in compaction (exclusive) |
| `kept_recent_count` | Yes | Number of recent raw turns preserved |
| `pre_compact_message_count` | Yes | Total message count before compaction |
| `pre_compact_estimated_tokens` | No | Estimated token count before compaction |
| `summary` | Yes | The rendered compact summary text (injection-ready) |

### 8.3 Schema Versioning

`schema_version` enables forward migration. When the schema changes:

1. Increment the version.
2. Add a migration path from old version to new.
3. Old versions are still readable but may trigger re-compaction on next access.

## 9. Integration Points

### 9.1 Modules to Modify

| Module | File | Change |
|---|---|---|
| **Session store** | `agent-diva-core/src/session/store.rs` | Add `compaction: Option<CompactSummary>` to `Session`; add `set_compaction()`, `clear_compaction()` |
| **Session history** | `agent-diva-core/src/session/store.rs` | Modify `get_history()` to return recent-only when compaction exists |
| **Context builder** | `agent-diva-agent/src/context.rs` | Add `build_messages_with_compaction()` that injects boundary + summary + recent turns |
| **Agent loop** | `agent-diva-agent/src/agent_loop/loop_turn.rs` | Insert context pressure check before building messages; call compactor if needed |
| **Consolidation** | `agent-diva-agent/src/consolidation.rs` | **No changes** — consolidation and compaction operate independently |

### 9.2 New Modules

| Module | Suggested location | Purpose |
|---|---|---|
| **ContextBudgetMonitor** | `agent-diva-agent/src/context_budget.rs` | Estimate context pressure, decide whether to compact |
| **ContextCompactor** | `agent-diva-agent/src/compaction.rs` | Run LLM compaction, parse structured summary |
| **CompactPrompt** | `agent-diva-agent/src/compaction/prompt.rs` | Compaction prompt templates |
| **TokenEstimator** | `agent-diva-agent/src/token_estimate.rs` | Character-to-token estimation with padding |

### 9.3 Files NOT Modified

| File | Reason |
|---|---|
| `agent-diva-core/src/memory/provider.rs` | MemoryProvider is the long-term memory boundary. Compaction does not cross it. |
| `agent-diva-agent/src/consolidation.rs` | Consolidation writes to MEMORY.md. Compaction does not. |
| `agent-diva-tools/*` | Tools are not affected by compaction. |

## 10. Boundaries and Non-Goals

### 10.1 Hard Boundaries

1. **Context compaction does NOT write MEMORY.md.** It does not call `MemoryProvider::sync_turn()`. Its output is session-local.
2. **Context compaction does NOT call `MemoryProvider::prefetch()`.** It does not interact with the recall system.
3. **Context compaction does NOT modify raw session messages.** Messages remain on disk. The compact summary is a separate overlay.
4. **Compact summaries are NOT authoritative.** The boundary marker explicitly states they are lossy and may contain inaccuracies.
5. **Compaction does NOT replace consolidation.** Both can run on the same session. Consolidation consumes raw messages for long-term memory; compaction summarizes them for session survival.

### 10.2 Relationship with MEMORY.md / MemoryProvider

```
Context Compaction              MemoryProvider Boundary
┌─────────────────┐            ┌─────────────────────────┐
│ Session-local    │            │ Long-term memory        │
│ Ephemeral        │            │ Durable                 │
│ Lossy summary    │            │ Curated facts           │
│ Never writes     │            │ MEMORY.md / HISTORY.md  │
│ MEMORY.md        │            │ via sync_turn()         │
│                  │            │                         │
│ Purpose:         │            │ Purpose:                │
│ keep session     │            │ preserve across         │
│ alive            │            │ sessions                │
└─────────────────┘            └─────────────────────────┘
        │                                │
        │  compaction may READ           │
        │  existing memory for           │
        │  context, but never WRITES     │
        └────────────────────────────────┘
```

### 10.3 Relationship with Consolidation (`consolidation.rs`)

| Dimension | Consolidation | Compaction |
|---|---|---|
| Purpose | Extract durable memory | Keep session alive |
| Output destination | MEMORY.md + HISTORY.md | Session.metadata.compaction |
| Trigger | `last_consolidated` gap ≥ 100 messages | Context pressure estimate |
| Uses LLM | Yes (to produce memory_update) | Yes (to produce structured summary) |
| Calls MemoryProvider | Yes (`sync_turn()`) | **No** |
| Survives session end | Yes (MEMORY.md persists) | No (session-local only) |
| `last_consolidated` reuse | Its own pointer | **Not reused** — uses separate compact checkpoint |

**Can they consume the same messages?** Yes, but for different purposes. Consolidation might process messages that were already compacted (the raw messages are still on disk). This is fine — consolidation reads raw messages for evidence, compaction reads them for working-state summary.

**Should `last_consolidated` be reused?** No. `last_consolidated` tracks memory consolidation progress. A compact checkpoint tracks what has been summarized for session survival. They have different semantics and different lifecycles.

### 10.4 Non-Goals

- **AutoDream rhythm system design**: Out of scope.
- **Journal UI**: Out of scope.
- **LearningCandidate pipeline**: Out of scope.
- **MEMORY.md long-term structure redesign**: Out of scope.
- **Provider major refactors**: Out of scope.
- **GUI product details**: Only relevant for manual `/compact` entry point.

## 11. Failure Modes

### 11.1 Compact LLM Call Fails

**Behavior**: Compaction is skipped. The session continues with the existing messages. If context is still over the limit, the next provider call may fail.

**Mitigation**: Log the error. The agent loop already handles provider errors gracefully. A future reactive compact (P1) can catch provider overflow and retry compaction.

### 11.2 Compact Summary Parse Failure

**Behavior**: The LLM returns a response that cannot be parsed into the expected structured format.

**Fallback**: Use the raw LLM response text as the summary (unstructured but better than nothing). Log a warning. Set a `parse_failed: true` flag in the compact metadata.

### 11.3 Provider Context Overflow After Compaction

**Behavior**: Even after compaction, the prompt is too large (e.g., system prompt + memory + skills + compact summary + recent turns exceeds the window).

**Mitigation**:
1. Reduce `keep_recent` count.
2. If still too large, truncate compact summary.
3. As ultimate fallback, alert the user that the session needs to be reset.

### 11.4 Repeated Compaction (Thrashing)

**Behavior**: Compaction triggers every turn because the context pressure never drops below threshold (e.g., large system prompt + memory block).

**Mitigation**: Circuit breaker — skip compaction after N consecutive compactions without meaningful pressure reduction (similar to Claude Code's `MAX_CONSECUTIVE_AUTOCOMPACT_FAILURES = 3`).

### 11.5 Summary Drift

**Behavior**: Over multiple compactions, the summary loses important details because each compaction summarizes the previous summary.

**Mitigation**:
1. Each new compaction includes the raw recent turns, not just the old summary.
2. The compact prompt explicitly instructs preservation of key facts, files, and decisions.
3. For P2: track a "key facts" list that accumulates across compactions rather than being re-summarized.

### 11.6 Compact During Active Tool Execution

**Behavior**: Compaction triggers while the agent is in the middle of a multi-step tool execution.

**Mitigation**: Only trigger compaction between complete turns (after `save_turn`, before the next `build_messages`). Never compact mid-tool-execution.

## 12. Testing Strategy

### 12.1 Unit Tests

| Test | What it validates |
|---|---|
| `estimate_session_tokens` | Character-to-token estimation accuracy within expected bounds |
| `should_compact` with message threshold | Triggers at exactly the configured message count |
| `should_compact` with token threshold | Triggers at the estimated token limit |
| `calculate_split_point` | Does not split tool_use/tool_result pairs |
| `parse_compact_summary` | Parses structured LLM output correctly |
| `parse_compact_summary_fallback` | Falls back to raw text on parse failure |
| `CompactSummary serialization` | Round-trip JSON serialize/deserialize |
| `build_messages_with_compaction` | Correct ordering: system → boundary → summary → recent → current |

### 12.2 Integration Tests

| Test | What it validates |
|---|---|
| Full compaction flow | Session with 100+ messages → compaction → prompt assembly → provider call succeeds |
| Compaction + consolidation coexistence | Both can run on the same session without conflict |
| Session persistence after compaction | Compact summary survives save/load cycle |
| Multiple compactions | Second compaction replaces first correctly |
| Reactive compaction | Provider overflow → emergency compaction → retry succeeds |

### 12.3 Regression Tests

| Test | What it validates |
|---|---|
| Sessions without compaction still work | No behavior change for short sessions |
| Consolidation still works after compaction | `last_consolidated` pointer is unaffected |
| MemoryProvider boundary integrity | No writes to MEMORY.md from compaction path |

## 13. MVP Plan

### P0: Core Context Compaction

**Goal**: Keep long sessions alive with a basic but correct compaction mechanism.

1. **`TokenEstimator`**: Character-based token estimation with 4/3 padding. Implement in `agent-diva-agent/src/token_estimate.rs`.
2. **`CompactSummary` schema**: Add to `agent-diva-core/src/session/store.rs` as part of `Session.metadata`. Implement `set_compaction()`, `clear_compaction()`, serialization.
3. **`ContextBudgetMonitor`**: Simple message-count + rough-token-estimate trigger. Implement in `agent-diva-agent/src/context_budget.rs`.
4. **`ContextCompactor`**: LLM-based compaction with structured prompt. Implement in `agent-diva-agent/src/compaction.rs`.
5. **Prompt assembly update**: Modify `ContextBuilder::build_messages()` to detect compaction state and inject boundary + summary + recent-only turns.
6. **Agent loop integration**: Insert budget check in `process_inbound_message_inner()` between `save_turn()` and the next turn's `build_messages()`.
7. **Tests**: Unit tests for all new modules; integration test for full compaction flow.

**Does not write MEMORY.md. Does not call MemoryProvider.**

### P1: Enhanced Triggers and Safety Nets

1. **Provider-aware context window**: Query provider for actual context window size; use ratio-based threshold.
2. **Manual `/compact` entry**: Expose compaction as a command/API for user-initiated compaction.
3. **Reactive compact**: Catch provider context-overflow errors and attempt emergency compaction.
4. **Compact events log**: Append compaction events to a session-local log for observability.
5. **Parse-failure fallback**: When structured parse fails, use raw text as summary with `parse_failed` flag.
6. **Post-compact file restore**: Re-inject recently accessed file paths (top 3-5) as context hints after compaction.

### P2: Advanced Features

1. **Multiple compaction merging**: Smarter handling of stacked compactions. Track accumulated "key facts" separately from narrative summary.
2. **Tool-state specialized summary**: Dedicated extraction of tool execution patterns and results.
3. **AutoDream evidence bridge**: Compact summaries can serve as evidence input for AutoDream's source capsule system, but AutoDream must still trace back to raw sessions. No direct write to long-term memory.
4. **Cross-session compact awareness**: When resuming a session, detect and load existing compact state.

## 14. Open Questions

### 14.1 Product Questions

1. **Manual compact UX**: Should `/compact` be a chat command, a CLI flag, a GUI button, or all three? Recommendation: start with chat command, add GUI button later.
2. **Compact notification**: Should the user be notified when auto-compaction happens? Recommendation: yes, a brief non-intrusive message (e.g., "Session context compressed to continue working").
3. **Compact history visibility**: Should users be able to see what was compacted? Recommendation: not in MVP; P1 could add a `/compact-info` command.

### 14.2 Architecture Questions

1. **Compaction LLM model**: Should compaction use the same model as the main conversation, or a cheaper/faster model? Recommendation: use the same model for quality; cost is a secondary concern for a single compression call per N turns.
2. **Compaction in subagent context**: If Agent-Diva spawns subagents (e.g., Mentle worker), should subagent sessions also compact? Recommendation: yes, but subagents can use a simpler trigger (message count only).
3. **Compact summary language**: Should the summary be in the user's language or always in the model's preferred language? Recommendation: same language as the conversation (detected from recent messages).
4. **Integration with Mentle**: Should compact summaries be searchable via Mentle? Recommendation: no in MVP. Compact summaries are ephemeral session state, not searchable knowledge.

### 14.3 Implementation Questions

1. **Compaction timing**: Should compaction happen before building the prompt (pre-build) or after a failed provider call (reactive)? Recommendation: pre-build as primary, reactive as safety net (P1).
2. **Async vs sync compaction**: Should the LLM compaction call block the response path? Recommendation: yes for MVP (compaction is the price of continuing). P1 could explore background pre-compaction.
3. **Storage migration**: If `Session.metadata` grows large with compact summaries, does it affect session load/save performance? Recommendation: monitor; if >100KB, consider external storage.

## Appendix A: File Reference Map

### Claude Code Reference Files

| File | Key content |
|---|---|
| `.workspace/claude-code/src/services/compact/compact.ts` | `compactConversation()`, `partialCompactConversation()`, boundary marker, post-compact file/skill/plan restore |
| `.workspace/claude-code/src/services/compact/autoCompact.ts` | `getAutoCompactThreshold()`, `shouldAutoCompact()`, `autoCompactIfNeeded()`, circuit breaker |
| `.workspace/claude-code/src/services/compact/prompt.ts` | `BASE_COMPACT_PROMPT`, `PARTIAL_COMPACT_PROMPT`, `formatCompactSummary()`, `<analysis>` + `<summary>` structure |
| `.workspace/claude-code/src/services/compact/microCompact.ts` | `microcompactMessages()`, tool result clearing, token estimation |
| `.workspace/claude-code/src/services/compact/snipCompact.ts` | Snip markers, message removal with boundary tracking |
| `.workspace/claude-code/src/services/compact/reactiveCompact.ts` | `tryReactiveCompact()`, prompt-too-long error handling |
| `.workspace/claude-code/src/services/compact/sessionMemoryCompact.ts` | Cross-session memory-aware compaction |
| `.workspace/claude-code/src/services/compact/grouping.ts` | `groupMessagesByApiRound()` for safe split points |
| `.workspace/claude-code/src/services/compact/postCompactCleanup.ts` | Cache/state cleanup after compaction |
| `.workspace/claude-code/src/services/autoDream/autoDream.ts` | AutoDream trigger, lock, forked agent — NOT context compaction |

### Agent-Diva Source Files

| File | Key content |
|---|---|
| `agent-diva-core/src/session/store.rs` | `Session`, `ChatMessage`, `get_history(50)`, `last_consolidated` |
| `agent-diva-core/src/session/manager.rs` | `SessionManager`, load/save/archive |
| `agent-diva-agent/src/context.rs` | `ContextBuilder`, `build_system_prompt()`, `build_messages()` |
| `agent-diva-agent/src/agent_loop/loop_turn.rs` | `process_inbound_message_inner()`, `save_turn()`, consolidation trigger |
| `agent-diva-agent/src/consolidation.rs` | `consolidate()`, `should_consolidate()`, MEMORY.md write |
| `agent-diva-core/src/memory/provider.rs` | `MemoryProvider` trait, 4 lifecycle hooks |
| `agent-diva-tools/src/sanitize.rs` | `truncate_tool_result()`, `MAX_TOOL_RESULT_CHARS` |

### Related Design Documents

| Document | Relevance |
|---|---|
| `docs/dev/genericagent/compression-taxonomy-decision.md` | Defines the three-way split: compaction vs consolidation vs rhythm distillation |
| `docs/dev/genericagent/compression-research.md` | Source Capsule design for AutoDream — distinct from this document |
| `docs/dev/genericagent/autodream-migration-research.md` | Claude Code AutoDream migration — context compaction is a prerequisite |
| `docs/dev/genericagent/newedge/architecture.md` | DivaGeneric overall architecture, L0-L4 layering |
| `docs/dev/archive/architecture-reports/openclaw-session-reset-analysis.md` | Session reset patterns — complementary to compaction |
| `docs/dev/archive/architecture-reports/上下文管理调研记录.md` | Earlier context management research, token-aware trimming direction |
