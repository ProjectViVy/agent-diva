# agent-diva Backend Session Storage Link Audit

**Date**: 2026-06-01
**Scope**: store.rs, manager.rs, loop_turn.rs, handlers.rs (+ consolidation.rs, loop_runtime_control.rs, runtime_control.rs)
**Type**: Deep Audit — bugs, concurrency, data-loss risk, consistency

---

## Architecture Overview

### Data Flow
```
User Message
  → handler::chat_handler (handlers.rs:45)
    → ManagerCommand::Chat → Manager::handle_chat (manager/runtime_control.rs:21)
      → bus.publish_inbound (async channel)
        → AgentLoop::process_inbound_message_inner (loop_turn.rs:19)
          → sessions.get_or_create(session_key)  [cache or disk → &mut Session]
          → session.get_history(50)              [read unconsolidated msgs]
          → context.build_messages(history, ...) [build LLM context Vec]
          → LLM loop (stream, tool_calls, iterations)
          → save_turn(session, ...)              [write turn to in-memory Session]
          → consolidation::consolidate(session)  [advance last_consolidated]
          → sessions.save(session)               [full overwrite JSONL file]
  → Manager API
    → handler::get_session_history_handler (handlers.rs:213)
      → ManagerCommand::GetSessionHistory
        → Manager::handle_get_session_history (manager/runtime_control.rs:103)
          → RuntimeControlCommand::GetSession (mpsc channel)
            → AgentLoop::handle_runtime_control_command (loop_runtime_control.rs:31)
              → sessions.get_or_load → clone → oneshot reply
```

### Key observation
Only ONE `SessionManager` instance exists — owned by `AgentLoop` (agent_loop.rs:103). All Manager API session access goes through async channel (`RuntimeControlCommand`) back to the same AgentLoop. This eliminates classic multi-threaded race conditions, but creates a different class of consistency problems.

---

## Bug #1 [P0] — User Message Never Written Before Turn Processing; Lost on Crash/Cancel

**File**: `agent-diva-agent/src/agent_loop/loop_turn.rs`
**Lines**: 62–64, 427–438, 624

**Description**:
The user's inbound message is loaded into the LLM context Vec at lines 69–74, but is NOT written to the Session until `save_turn()` at line 431–438. The `save_turn()` call at line 624 executes `session.add_message(user_role, user_content)`.

Between the session load (line 64) and save_turn (line 431), there are multiple early-return paths:
- Cancellation at iteration start (lines 123–126) → `return Ok(None)`
- Cancellation during streaming (lines 179–182) → `return Ok(None)`
- Cancellation during tool execution (lines 279–282) → `return Ok(None)`
- LLM returns error finish_reason (lines 383–387) → `break` then fall-through

In ALL these cases, `save_turn` is never reached. The user message is permanently lost — it was never written to the session and never will be.

**Impact**: Silent data loss of all user input on any error, cancellation, or crash before turn completion.

**Severity**: P0 — User messages are the primary data; every lost message degrades the assistant's memory.

**Fix**: Write the user message to the session immediately after `get_or_create()` at line 64, before starting the LLM loop. Save it to disk right after.

---

## Bug #2 [P0] — Consolidation Advances `last_consolidated` Before Disk Save; Crash Gap

**File**: `agent-diva-agent/src/consolidation.rs` line 190, `agent-diva-agent/src/agent_loop/loop_turn.rs` lines 441–465

**Description**:
The execution order in `process_inbound_message_inner`:
```
Line 427–438: save_turn(session, ...)          // messages added to session
Line 441–458: consolidation::consolidate(...)  // last_consolidated advanced (consolidation.rs:190)
Line 461–465: sessions.save(session)           // disk persistence
```

If the process crashes (panic, kill, OOM) between line 458 and line 465:
1. Session in-memory has new messages AND advanced `last_consolidated` — lost on restart
2. Memory provider (`MEMORY.md`/`HISTORY.md`) has consolidation results — persisted
3. Session JSONL on disk: has NEITHER the new messages NOR the updated `last_consolidated`

On next startup: the session reloads from disk with the OLD `last_consolidated`, so the same messages will be consolidated again → **double consolidation**. The new turn's messages are gone forever.

Additionally: memory provider's `sync_turn` (consolidation.rs:149–177) has already written to `MEMORY.md`. The memory claims to have "consolidated" messages that in fact haven't been marked as consolidated in the session — leading to a split-brain between memory store and session state.

**Impact**: Double consolidation + permanent message loss on crash after consolidation but before save.

**Severity**: P0 — This gap exists on every turn that triggers consolidation.

**Fix**: Save the session to disk BEFORE running consolidation, or ensure consolidation's side effects are rolled back if save fails.

---

## Bug #3 [P0] — JSONL Full Overwrite Is Not Atomic; Crash During Write = Total Corruption

**File**: `agent-diva-core/src/session/manager.rs`, lines 103–126

**Description**:
```rust
pub fn save(&self, session: &Session) -> crate::Result<()> {
    // ... build lines ...
    std::fs::write(&path, lines.join("\n"))?;  // line 124
    Ok(())
}
```

`std::fs::write` truncates the file and writes new content. If the process crashes mid-write:
- The file is left partially written or empty
- The old complete data is irreversibly lost (no backup, no temp file)

This is especially dangerous given Bug #2 (consolidation before save): the disk file is the ONLY durable copy. No atomic write-then-rename pattern is used.

**Impact**: On crash, the entire session history can be corrupted or zeroed out.

**Severity**: P0

**Fix**: Write to a `.tmp` file first, then `fs::rename` (which is atomic on most filesystems).

---

## Bug #4 [P0] — `save()` Failure Silently Discards Turn Data With No Retry

**File**: `agent-diva-agent/src/agent_loop/loop_turn.rs`, lines 461–465

**Description**:
```rust
if let Some(session) = self.sessions.get(&session_key) {
    if let Err(e) = self.sessions.save(session) {
        error!("Failed to save session: {}", e);
    }
}
```

If `save()` fails (disk full, permission error, I/O error), the entire turn's data — user message, assistant response, tool results, and consolidation advancement — exists ONLY in the in-memory cache. The error is logged but:
1. No retry is attempted
2. The caller returns `Ok(Some(outbound_message))` — the API reports success
3. On next restart, all this turn's data is lost
4. The user received a response but the session has no record of it

Combined with Bug #1, this means even a transient disk error can cause permanent data loss.

**Impact**: Session silently diverges from what the user experienced; data lost on restart.

**Severity**: P0

**Fix**: Retry with backoff; if retry fails, mark the session as "dirty" and attempt save on next turn or shutdown.

---

## Bug #5 [P1] — `load()` Silently Returns None on I/O Error; Old History Destroyed

**File**: `agent-diva-core/src/session/manager.rs`, lines 56–63

**Description**:
```rust
fn load(&self, key: &str) -> Option<Session> {
    let path = self.session_path(key);
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(&path).ok()?;
    // ...
}
```

If `read_to_string` fails (I/O error, permission denied, file locked), `.ok()?` silently converts it to `None`. Then `get_or_create` (line 31) creates a NEW empty session:
```rust
let session = self.load(&key).unwrap_or_else(|| Session::new(&key));
```

The new empty session will eventually be saved back to disk (overwriting the corrupted/unreadable file), destroying any recoverable data.

**Impact**: A transient I/O error can cause permanent loss of all session history.

**Severity**: P1

**Fix**: Distinguish "file not found" from "file unreadable". Log an error and refuse to overwrite on read failure.

---

## Bug #6 [P1] — JSONL Parsing Silently Drops Unparseable Lines

**File**: `agent-diva-core/src/session/manager.rs`, lines 69–90

**Description**:
```rust
for line in content.lines() {
    let line = line.trim();
    if line.is_empty() { continue; }
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(line) {
        if value.get("_type")... == Some("metadata") { ... }
        else if let Ok(msg) = serde_json::from_value::<...::ChatMessage>(value) { ... }
        // ELSE: valid JSON that is neither metadata nor ChatMessage → SILENTLY DROPPED
    }
    // ELSE: invalid JSON line → SILENTLY DROPPED
}
```

Lines that are valid JSON but fail `ChatMessage` deserialization (e.g., missing `timestamp` field after schema change) are silently dropped. No warning, no error count. Messages disappear without trace.

**Impact**: Silent data loss on malformed JSONL lines — no observability.

**Severity**: P1

**Fix**: Count and log dropped lines. Consider preserving unknown JSON types as a `serde_json::Value` fallback.

---

## Bug #7 [P1] — `list_sessions()` Key Encoding Is Not Reversible

**File**: `agent-diva-core/src/session/manager.rs`, lines 167, 202–205

**Description**:
`session_path()` encodes keys: `:`, `/`, `\` are all replaced with `_` (line 203):
```rust
let safe_key = key.replace([':', '/', '\\'], "_");
```

`list_sessions()` decodes keys: ALL `_` are replaced with `:` (line 167):
```rust
let key = name.trim_end_matches(".jsonl").replace('_', ":");
```

**Counterexample**: key `telegram:user_123`
- Encoded filename: `telegram_user_123.jsonl`
- Decoded key: `telegram:user:123` (WRONG — `_` in `user_123` became `:`)

Also: keys containing `/` or `\` cannot be round-tripped at all.

The handler in `handlers.rs:219` adds `gui:` prefix for IDs without `:` — a key with underscores from a non-gui channel (e.g., `discord-general_chat`) would also get a spurious `gui:` prefix, because `_` is not `:`.

**Impact**: `list_sessions()` returns incorrect keys for sessions with underscores. Session lookups via the Manager API may fail or retrieve wrong sessions.

**Severity**: P1

**Fix**: Use a lossless encoding (e.g., URL-encoding) or store the original key in the metadata line.

---

## Bug #8 [P1] — Prefetch Injection Shifts Message Index, Breaking `save_turn` Offset Calculation

**File**: `agent-diva-agent/src/agent_loop/loop_turn.rs`, lines 109, 627

**Description**:
At line 109, prefetch recall injects an extra system message:
```rust
messages.insert(1, agent_diva_providers::Message::system(block));
```

The `save_turn` offset calculation at line 627 assumes exactly 1 system prompt:
```rust
let turn_start = 1 + history_len + 1;
//                ^ system prompt
//                     ^ history messages
//                           ^ user message
```

After prefetch injection, the layout becomes:
```
[0] system prompt
[1] prefetch recall system message    ← EXTRA
[2..2+history_len] history messages
[2+history_len] user message
```

But `turn_start` is still `1 + history_len + 1 = history_len + 2`, which now points to the LAST history message (or the user message, depending on exact lengths). This causes `save_turn` to either:
- Include history messages as "new turn" messages (duplication)
- Skip the first assistant/tool messages of the actual turn (data loss)

**Impact**: Session corruption when prefetch is active — saved messages include old history or miss new turn messages.

**Severity**: P1

**Fix**: Compute `turn_start` dynamically by counting messages from the end, or tag messages with a "turn_id".

---

## Bug #9 [P1] — Cron Messages Saved as "system" Role, Excluded from `get_history()`

**File**: `agent-diva-agent/src/agent_loop/loop_turn.rs` line 430, `agent-diva-core/src/session/store.rs` line 67

**Description**:
For cron-triggered turns, the user message is saved with role `"system"` (loop_turn.rs:430):
```rust
let user_role = if is_cron_trigger { "system" } else { "user" };
session.add_message(user_role, user_content);
```

But `get_history()` explicitly filters to only `"user" | "assistant" | "tool"` (store.rs:67):
```rust
.filter(|m| matches!(m.role.as_str(), "user" | "assistant" | "tool"))
```

Cron-triggered messages are completely invisible to the LLM in future turns. Additionally, the leading-user-message filter (line 71–73) will skip system messages and start from the first actual user message — which may skip real assistant/tool context if there are no user messages in the window.

**Impact**: The LLM loses context of what cron jobs triggered, degrading cron-based automation quality.

**Severity**: P1

**Fix**: Add `"system"` to the filter in `get_history()`. Or save cron messages with `"user"` role and note the cron origin in metadata.

---

## Bug #10 [P1] — Stale In-Memory Cache After Disk Write Failure

**File**: `agent-diva-core/src/session/manager.rs`

**Description**:
The in-memory cache (`HashMap<String, Session>`) is updated via `get_or_create()` which returns `&mut Session`. Mutations to the session (from `save_turn`, `consolidate`) affect the cache directly. If `save()` fails (Bug #4), the cache diverges from disk:
- Cache: has the turn's messages + advanced consolidation index
- Disk: has neither

On next `get_or_create` for the same key: cache hit returns the mutated (unsaved) session. This appears correct from the agent's perspective during the same process lifetime, but:
- Any restart loses the data permanently
- No mechanism exists to detect or reconcile the divergence

**Impact**: False sense of durability — the system behaves as if data is saved when it isn't.

**Severity**: P1

**Fix**: Track a "dirty" flag per session; retry saves; warn on shutdown if dirty sessions exist.

---

## Bug #11 [P2] — `get_history()` May Return Orphaned Tool/Assistant Messages After Consolidation

**File**: `agent-diva-core/src/session/store.rs`, lines 60–75

**Description**:
```rust
let mut sliced: Vec<ChatMessage> = unconsolidated[start..]
    .iter()
    .filter(|m| matches!(m.role.as_str(), "user" | "assistant" | "tool"))
    .cloned()
    .collect();
if let Some(pos) = sliced.iter().position(|m| m.role == "user") {
    sliced = sliced[pos..].to_vec();
}
```

If the unconsolidated window contains ONLY assistant/tool messages (e.g., after a consolidation boundary falls in the middle of a tool-use sequence), the `if let Some(pos)` branch is NOT taken. The original `sliced` (with orphaned tool/assistant messages but no user message) is returned as-is.

The LLM receives orphaned tool results without context — this can cause confusion, hallucinations, or errors.

**Impact**: Degraded LLM context quality after consolidation boundaries.

**Severity**: P2

**Fix**: If no user message is found, return an empty Vec or search backward into the consolidated range.

---

## Bug #12 [P2] — Tool Results Truncated to 500 Characters on Save

**File**: `agent-diva-agent/src/agent_loop/loop_turn.rs`, lines 659–663

**Description**:
```rust
let content = if m.content.chars().count() > 500 {
    format!("{}...", m.content.chars().take(500).collect::<String>())
} else {
    m.content.clone()
};
```

Tool results beyond 500 characters are truncated with "..." appended. For tools like `read_file`, `web_fetch`, `shell`, or `list_dir`, the output often exceeds 500 characters. The truncated data is permanently lost from the session context. Future turns can only see the first 500 chars of any tool result.

**Impact**: LLM loses access to full tool outputs in conversation history.

**Severity**: P2

**Fix**: Raise the limit (or make it configurable), use a summary for very long outputs, or store a reference to a file.

---

## Bug #13 [P2] — `delete()` Cache Removal Before File Deletion

**File**: `agent-diva-core/src/session/manager.rs`, lines 129–138

**Description**:
```rust
pub fn delete(&mut self, key: &str) -> crate::Result<bool> {
    self.cache.remove(key);         // cache removed FIRST
    let path = self.session_path(key);
    if path.exists() {
        std::fs::remove_file(&path)?;  // if this fails...
        Ok(true)
    } else {
        Ok(false)
    }
}
```

If `remove_file` fails (permissions, file locked):
- `?` propagates the error
- Cache entry is already removed
- File remains on disk

Next `get_or_create` → cache miss → `load()` succeeds (file exists) → session resurrected from disk. The "deletion" was effectively rolled back in a confusing way.

**Impact**: Partial deletion state — confusing but not permanently destructive.

**Severity**: P2

**Fix**: Remove cache entry only after successful file deletion. Same issue in `archive_and_reset()` (lines 142–157).

---

## Bug #14 [P2] — `get_session_history_handler` Prefix Fallback Breaks Non-GUI Channels

**File**: `agent-diva-manager/src/handlers.rs`, lines 219–223

**Description**:
```rust
let session_key = if !id.contains(':') {
    format!("gui:{}", id)  // hardcoded "gui:" prefix
} else {
    id
};
```

If a frontend sends just a chat_id without the channel prefix (e.g., `"general_chat"` from Discord), the handler prepends `"gui:"`, creating `"gui:general_chat"`. This key will never match `"discord:general_chat"` on disk. The API returns "Session not found".

Additionally, since `_` is used as the encoding character for `:` (Bug #7), a Discord chat_id `"general_chat"` has no `:` in it, so it gets the `gui:` prefix incorrectly.

**Impact**: API returns 404 for valid sessions from non-GUI channels.

**Severity**: P2

**Fix**: Require the full `channel:chat_id` format from the frontend, or pass channel as a separate query parameter.

---

## Bug #15 [P2] — `save_turn` Final Message Detection May Miss Content

**File**: `agent-diva-agent/src/agent_loop/loop_turn.rs`, lines 632–639, 678

**Description**:
Empty assistant messages (no content + no tool_calls) are skipped in the loop at line 636. The final check at line 678:
```rust
if messages.len() <= turn_start || messages.last().map(|m| m.role.as_str()) != Some("assistant")
```
only checks the last message's role. If a skipped empty assistant message is followed by tool results, the last message is "tool" → triggers final_content append. But if an empty assistant message IS the last message (no tool calls followed), line 678 would see `Some("assistant")` and NOT append final_content.

However, in practice, if the assistant message has no content and no tool_calls, the loop breaks with final_content set (line 389-391), and the last message in `messages` is NOT assistant — it's the user message (since no assistant message was added). So the check works correctly in the normal flow.

Edge case: If `add_assistant_message` is called (line 268-274) with empty content but tool_calls present, the message gets tool_calls → NOT skipped (line 632-636 check passes), so it gets saved. Good.

**Impact**: Low — edge case unlikely to trigger in practice.

**Severity**: P2

---

## Summary Table

| # | Bug | Severity | File(s) | Data Loss? |
|---|-----|----------|---------|------------|
| 1 | User msg not written before turn; lost on cancel/crash | P0 | loop_turn.rs | YES |
| 2 | Consolidation before save; crash = split-brain | P0 | loop_turn.rs, consolidation.rs | YES |
| 3 | JSONL overwrite not atomic; crash = corruption | P0 | manager.rs | YES |
| 4 | save() failure silently ignored; no retry | P0 | loop_turn.rs | YES |
| 5 | load() error → silent None → new empty session | P1 | manager.rs | YES |
| 6 | JSONL parsing silently drops invalid lines | P1 | manager.rs | YES |
| 7 | list_sessions key encoding irreversible | P1 | manager.rs | No (wrong keys) |
| 8 | Prefetch injection breaks save_turn offsets | P1 | loop_turn.rs | YES |
| 9 | Cron messages excluded from get_history() | P1 | loop_turn.rs, store.rs | Context loss |
| 10 | Stale cache after save failure | P1 | manager.rs | YES (on restart) |
| 11 | Orphaned tool msgs after consolidation | P2 | store.rs | Context degradation |
| 12 | Tool results truncated to 500 chars | P2 | loop_turn.rs | Partial |
| 13 | Delete cache before file removal | P2 | manager.rs | Transient |
| 14 | GUI prefix fallback in handler | P2 | handlers.rs | Lookup failure |
| 15 | Final msg detection edge case | P2 | loop_turn.rs | Rare |

---

## Data Flow Diagram

```
USER INPUT (HTTP/bot)
    │
    ▼
┌──────────────────────────────────────────┐
│ handlers.rs: chat_handler                 │
│   channel = payload.channel || "api"     │
│   chat_id = payload.chat_id || "default"  │
│   → ManagerCommand::Chat(ApiRequest)      │
└────────────┬─────────────────────────────┘
             │ mpsc::Sender
             ▼
┌──────────────────────────────────────────┐
│ Manager::handle_chat (runtime_control.rs) │
│   → bus.publish_inbound(InboundMessage)   │
└────────────┬─────────────────────────────┘
             │ broadcast/mpsc
             ▼
┌──────────────────────────────────────────┐
│ AgentLoop::process_inbound_message_inner  │
│                                            │
│  1. session_key = "{channel}:{chat_id}"   │  ← KEY FORMAT
│  2. sessions.get_or_create(&session_key)  │  ← LOAD (cache→disk→new)
│  3. history = session.get_history(50)     │  ← READ (unconsolidated, filter, truncate leading non-user)
│  4. messages = context.build_messages(...) │  ← BUILD LLM CONTEXT
│  5. LLM loop (stream + tools)             │  ← PROCESS
│  6. save_turn(session, messages, ...)     │  ← WRITE TO CACHE (user+assistant+tool msgs)
│  7. consolidation::consolidate(session)   │  ← ADVANCE last_consolidated
│  8. sessions.save(session)                │  ← PERSIST TO DISK (full overwrite)
└────────────┬─────────────────────────────┘
             │
    ┌────────┴────────┐
    ▼                 ▼
┌───────────┐  ┌──────────────┐
│  CACHE    │  │  DISK (JSONL) │
│ HashMap   │  │  {key}.jsonl  │
│ <String,  │  │  line1: meta  │
│  Session> │  │  line2+: msgs │
└───────────┘  └──────┬───────┘
                       │
         ┌─────────────┴─────────────┐
         ▼                           ▼
┌──────────────────┐     ┌──────────────────────┐
│ Manager API Read │     │ Next AgentLoop Start │
│ (GetSessionHist) │     │ (get_or_create load) │
│   ↓              │     │   ↓                  │
│ RuntimeControl   │     │ load() → Session     │
│ Command → Agent  │     │ from disk            │
│ Loop → get_or_   │     └──────────────────────┘
│ load → Clone →   │
│ JSON response    │
└──────────────────┘
```

### Critical Paths Noted:
- **Red path**: Steps 6→7→8 — Bug #2 gap (consolidation before save)
- **Orange path**: Step 6 — Bug #1 gap (user msg not in session until here)
- **Dashed line**: API read goes through AgentLoop's mpsc channel — single-threaded but may block

---

## Additional Observability Concerns

1. **No metrics on session operations**: No counters for saves, loads, parse failures, or dropped messages.
2. **No session size limits**: Sessions grow unbounded; `get_history` only takes last N, but all messages are loaded into memory on every `load()`.
3. **No backup mechanism**: Single JSONL file per session with no automatic backup or WAL.
4. **No migration layer**: If `ChatMessage` schema changes, old JSONL files fail to parse and messages are silently dropped (Bug #6).
5. **Shutdown race**: If the process is killed during `run()` (line 373 agent_loop.rs), the in-memory cache is lost because agent loop owns the SessionManager. No shutdown hook flushes all sessions.
