# Phase A-PRE: Session Truth Source And Durability Fix

## Background And Goal

Before implementing Phase A (`Plan + TodoList`), agent-diva must fix the
session truth-source and durability boundary.

The backend JSONL/session history is intended to be the durable source of truth,
but the current implementation has two related problems:

1. backend session persistence happens too late in the agent turn and is not
   atomic enough for crash/error recovery;
2. the GUI can prefer front-end `localStorage` cache over backend history.

These combine into a serious consistency risk. The GUI can show stale history
for up to the cache TTL, and the backend can lose a user message if the turn is
canceled, fails, or crashes before the end-of-turn save path.

There is a second consistency boundary during message send: the GUI may
optimistically push the user message and an empty assistant placeholder before
the backend has persisted the agent turn. If the turn fails mid-flight, the GUI
render state and backend persisted state may diverge temporarily or permanently.

Phase A-PRE fixes this foundation:

```text
Backend session history is authoritative.
User input is persisted before long-running turn execution.
Frontend cache is only a backend-unavailable fallback, not a competing source of truth.
```

## Why This Must Precede Plan + TodoList

Plan/Todo execution will depend on the current conversation state and GUI
display. If the GUI can resurrect stale cached messages, it can show the wrong
context around an active plan, TodoList, verification failure, or recovery
prompt.

This is not a polish task. It is a state consistency and durability
prerequisite.

Plan/Todo must not be built on top of a session layer where:

- a user instruction can disappear after cancellation;
- GUI history can diverge from backend JSONL;
- reset/delete/switch operations can resurrect stale cached messages;
- observability cannot distinguish persisted history from optimistic render
  state.

## Non Goals

- No Plan/Todo implementation.
- No large backend session format migration unless required by the fix.
- No full append-only transcript rewrite in this phase.
- No large GUI rewrite.
- No new chat UI design.
- No offline-first session model.
- No conflict-free replicated data type or bidirectional sync system.
- No replay engine.
- No Kanban/task-board work.

## Validated Research Update

This section incorporates the reviewed findings from:

- `docs/logs/2026-06-session-research/session-history-storage-research.md`
- `docs/logs/2026-06-session-research/subagent-backend-audit.md`
- `docs/logs/2026-06-session-research/subagent-gui-audit.md`
- `docs/logs/2026-06-session-research/subagent-reference-comparison.md`

The core conclusion is valid: short-term conversation history has real
durability and consistency bugs. However, the raw research output should not be
copied directly into implementation scope. Some severity labels were too high,
and the reference comparison report contains at least one stale conclusion:
agent-diva does have backend JSONL session persistence today, so it is not a
pure in-memory session system.

### Confirmed P0 Findings

These findings are directly supported by current code and should gate Phase A:

| Finding | Evidence | Phase A-PRE Action |
| --- | --- | --- |
| User message is not persisted before turn execution | `process_inbound_message_inner` builds context after `sessions.get_or_create`, but `session.add_message` happens inside `save_turn` near the end of the turn | Persist the inbound user message immediately after session load, before LLM/tool execution |
| Cancel/stream/provider failure can skip `save_turn` | Several early `return Ok(None)` paths occur before the end-of-turn save path | Ensure the user message is already durable before those paths |
| Session save is full-file non-atomic overwrite | `SessionManager::save` serializes all lines and calls `std::fs::write` | Use temp-file write plus rename, or another atomic write strategy |
| `save()` failure is logged but does not change success flow | Agent loop logs `Failed to save session` and still returns a response | Add retry/dirty-state/error visibility so persistence failure cannot be silently treated as durable success |
| Consolidation runs before session save | `consolidation::consolidate` can write memory updates and advance `last_consolidated` before `sessions.save` | Persist the raw turn before consolidation, or make consolidation transactional with session save |
| GUI `loadSession()` is cache-first | `readSessionFromCache` is called before `get_session_history` | Make backend read the default; cache only fallback on backend failure |
| GUI session cache is not invalidated/refreshed after send completion | `writeSessionToCache` is only used from `loadSession` backend miss path | Refresh/clear cache only from canonical backend history |
| Optimistic UI can leave non-authoritative messages/placeholders | `sendMessage` pushes user message and assistant placeholder before backend persistence; error path only partially cleans state | Mark optimistic state as local/pending and reconcile with backend canonical history |

### Confirmed P1 Findings

These are important, but they should not block the first P0 repair slice unless
they are cheap to fix in the same files:

- `SessionManager::load` collapses I/O/read errors into `None`; this can cause a
  new empty session to overwrite recoverable old data.
- JSONL parsing silently drops malformed or schema-incompatible lines.
- `session_path` encoding is not reversible because `:`, `/`, and `\` all map to
  `_`, while `list_sessions` maps every `_` back to `:`.
- Prefetch system-message injection can shift the `save_turn` offset because
  `turn_start` assumes exactly one system prompt plus history plus current user
  message.
- Cron-triggered messages are saved with `system` role but `get_history()`
  filters to `user`, `assistant`, and `tool`.
- GUI delete failure currently removes/hides the session locally even when
  backend deletion failed.
- GUI `stopMessage` does not reconcile from backend canonical history.

### Downgraded Or Rejected Findings

These findings should be treated carefully:

- `activeStreamRequestId` overwrite is not P0 in the normal GUI path because
  `sendMessage` exits when `isTyping` is true. It is still a race-prone area,
  but should be P1/P2 unless a reproducible bypass exists.
- "Tool error causes user message loss" is too broad. Tool execution errors are
  usually converted into tool result strings and can still reach the save path.
  Cancellation, stream/provider error, crash, and save failure are the stronger
  data-loss paths.
- "LLM finish_reason error skips persistence" is not supported by current code;
  the loop creates an apology response and continues to the normal save path.
- "agent-diva has no session persistence" in the reference comparison is stale
  or incorrect. Current backend session persistence is JSONL full-file rewrite.

## Current Risk Model

### Risk 1: User Message Is Not Durable Before Turn Work

Current concern:

```text
inbound user message
  -> session is loaded
  -> LLM/tool loop runs
  -> cancellation, provider error, stream error, or crash happens
  -> save_turn is never reached
  -> user message was never written to backend history
```

Impact:

- User input can disappear from short-term history.
- Resume/recovery cannot reconstruct the failed turn.
- Plan/Todo may miss the instruction that started or changed a plan.

### Risk 2: Non-Atomic Session Save Can Corrupt Or Lose History

Current concern:

```text
SessionManager::save(session)
  -> serialize metadata + all messages
  -> std::fs::write(path, full_jsonl)
  -> process crashes or write fails
```

Impact:

- Existing session JSONL can become partial or empty.
- In-memory session can diverge from disk.
- User sees a successful response even though the turn was not durably saved.

### Risk 3: Consolidation Split-Brain

Current concern:

```text
save_turn mutates in-memory session
  -> consolidation writes MemoryProvider output
  -> consolidation advances last_consolidated
  -> session save fails or process crashes
```

Impact:

- Memory/HISTORY may claim a segment was consolidated.
- Session JSONL may not contain the same turn or updated consolidation pointer.
- Restart can cause duplicate consolidation or missing session context.

### Risk 4: Stale GUI Cache Wins Over Backend

Current concern:

```text
loadSession(session_id)
  -> reads agent-diva-session-cache:<session_id>
  -> cache is still within 30 minutes
  -> GUI renders cached messages
  -> backend may already have newer JSONL history
```

Impact:

- User sees stale chat history.
- Plan/Todo state can appear missing or outdated.
- Reset/delete/new session can appear to be undone by cache resurrection.
- Debugging becomes misleading because GUI state does not match backend state.

### Risk 5: Optimistic Placeholder Becomes Sticky

Current concern:

```text
send message
  -> frontend pushes user message
  -> frontend pushes empty assistant placeholder
  -> backend persists only after agent turn completes
  -> turn fails/interrupted
  -> frontend placeholder may remain as if it were part of history
```

Impact:

- GUI displays non-authoritative local messages.
- Empty assistant turns may persist in local cache.
- Refresh or session switch can produce confusing history.

## Target Behavior

### Persist User Message Before Turn Execution

Default behavior:

```text
process inbound message:
  1. load/create backend session
  2. append inbound user message with turn/request id
  3. persist the session immediately
  4. execute LLM/tool loop
  5. append assistant/tool outputs
  6. persist final turn state
  7. run consolidation only after raw turn persistence is durable
```

If step 3 fails, the system must not pretend the turn is safely durable. The
MVP can either return a visible error or mark the session dirty and surface an
explicit persistence warning.

### Save Session Atomically

Default behavior:

```text
save(session):
  1. serialize complete JSONL to a temporary file in the same directory
  2. flush/sync enough for the platform target
  3. rename temporary file over the target file
  4. preserve the old file if serialization or write fails
```

This phase does not require switching to append-only JSONL. Append-only remains
a possible later durability upgrade, but Phase A-PRE only needs to remove the
current full-file partial-write hazard.

### Load Or Parse Failures

Default behavior:

```text
load(session_key):
  1. file not found -> None
  2. file exists but cannot be read -> error, do not create replacement session
  3. malformed line -> visible warning/error counter
  4. unrecoverable corruption -> preserve file, do not overwrite silently
```

This protects old session history from being overwritten by an accidental empty
session after a transient read or parse failure.

### Load Session

Default behavior:

```text
loadSession(session_id):
  1. fetch backend session history
  2. if backend succeeds:
       replace frontend messages with backend history
       refresh local cache from backend history
       clear any stale pending placeholders for that session
  3. if backend fails:
       fallback to localStorage cache
       mark UI as cache fallback / possibly stale
```

Frontend cache must never override a successful backend read. If backend returns
an empty or not-found response, cache fallback should be used only when the
backend request itself failed or is explicitly unavailable, not when the backend
successfully says the session does not exist.

### Send Message

Optimistic UI is still allowed, but it must be explicitly local:

```text
sendMessage(input):
  1. append local pending user message
  2. append local pending assistant placeholder only if needed for streaming UI
  3. send to backend
  4. on completion:
       replace messages from backend canonical history
       invalidate or refresh local cache
  5. on failure:
       remove empty assistant placeholder or mark failed
       keep user-visible error state
       do not promote localOnly messages to authoritative history
```

Successful completion should prefer a backend re-fetch or a canonical response
from the backend over manually merging stream-rendered messages into cache.

### Session Mutations

These operations must invalidate matching cache keys:

- successful send completion
- failed send completion
- delete session
- reset session
- new session switch
- explicit session switch
- backend history revision/updated_at mismatch

The target cache key family is:

```text
agent-diva-session-cache:*
```

## Code Map

Relevant current files:

- `agent-diva-agent/src/agent_loop/loop_turn.rs`
  - `process_inbound_message_inner` loads session, builds context, executes the
    LLM/tool loop, calls `save_turn`, runs consolidation, and saves the session.
  - `save_turn` appends the user message and assistant/tool outputs only near
    the end of the turn.
  - The `turn_start` calculation currently assumes one system prompt.
- `agent-diva-agent/src/consolidation.rs`
  - Writes memory/HISTORY updates and advances `last_consolidated`.
- `agent-diva-core/src/session/manager.rs`
  - `SessionManager::load`
  - `SessionManager::save`
  - `SessionManager::delete`
  - `SessionManager::list_sessions`
  - `session_path`
- `agent-diva-core/src/session/store.rs`
  - `Session`
  - `ChatMessage`
  - `get_history`
- `agent-diva-manager/src/handlers.rs`
  - `get_session_history_handler`
  - `delete_session_handler`
  - fallback prefixing of bare IDs as `gui:<id>`.
- `agent-diva-manager/src/manager/runtime_control.rs`
  - Manager control path for session history retrieval/deletion.
- `agent-diva-gui/src/App.vue`
  - `SESSION_CACHE_TTL_MS`
  - `readSessionFromCache`
  - `writeSessionToCache`
  - `loadSession`
  - `sendMessage`
  - `stopMessage`
  - `deleteSession`
  - streaming event handlers.

## Design Requirements

### Durable Inbound Message

The inbound user message must be written to backend session history before
provider calls or tool execution.

Requirements:

- avoid duplicating the user message when final turn output is appended;
- include enough metadata to correlate pending GUI state, stream request id, and
  backend persisted turn;
- if immediate persistence fails, surface the failure instead of silently
  continuing as if the turn is durable.

### Atomic Backend Save

`SessionManager::save` must not corrupt an existing session file when a write
fails midway.

Requirements:

- write temp file in the same directory;
- rename into place only after successful serialization/write;
- clean up stale temp files best-effort;
- keep old target file if new write fails.

### Load Failure Visibility

`SessionManager::load` should distinguish:

- file missing;
- file unreadable;
- malformed line;
- schema mismatch.

The first implementation can keep the public API small, but it must prevent the
"read failed -> create empty session -> overwrite old file" path.

### Backend-First Loading

`loadSession()` must attempt backend history first. Cache fallback is only
allowed when backend history cannot be fetched.

### Canonical Replacement

After backend success, the GUI should replace the current message array with the
canonical backend history rather than merge blindly with local cache.

### Pending Message Metadata

Optimistic messages should carry local-only metadata:

```ts
type MessageRenderState = {
  pending?: boolean;
  localOnly?: boolean;
  failed?: boolean;
  backendMessageId?: string;
};
```

The exact type may differ; the important behavior is that local-only messages
are not treated as persisted backend history.

### Cache Invalidation

Add a narrow helper if missing:

```ts
invalidateSessionCache(sessionId?: string): void
```

Behavior:

- With `sessionId`, remove only that session cache key.
- Without `sessionId`, remove every `agent-diva-session-cache:*` key.

Cache invalidation is not a replacement for backend-first loading. It is a
secondary guardrail.

### Cache Fallback UI

If backend is unavailable and local cache is used, the UI should make the state
visible:

```text
Showing local cached session history. Backend history is unavailable; this view may be outdated.
```

The exact copy can be localized later. The MVP can use existing notification
patterns.

## Implementation Options

### Option A: GUI-Only Backend-First Swap

Change `loadSession()` order only:

1. Fetch backend.
2. Fallback cache only on backend error.
3. Refresh cache on backend success.

Pros:

- Smallest patch.
- Fixes stale cache priority.

Cons:

- Does not solve backend data loss.
- Does not solve non-atomic session save.
- Does not fully solve sticky optimistic placeholders.

Not sufficient as Phase A-PRE.

### Option B: Backend Durability Plus GUI Reconciliation

Fix backend durability first, then normalize GUI loading and optimistic state:

1. Persist inbound user message before turn execution.
2. Make session save atomic and persistence failures visible.
3. Save raw turn before consolidation.
4. Backend-first `loadSession()`.
5. Pending/localOnly markers for optimistic messages.
6. Re-fetch or canonical replace after turn completion.
7. Cache invalidation on send/reset/delete/switch.

Pros:

- Fixes both backend and GUI sources of inconsistency.
- Good foundation for Plan/Todo.

Cons:

- Requires touching agent loop, session manager, and chat send flow.

Recommended.

### Option C: Append-Only Session Transcript

Replace full-file JSONL rewrite with append-only JSONL and a write queue.

Pros:

- Strongest long-term transcript semantics.
- Aligns with claude-code-style durability.

Cons:

- Larger migration.
- More moving parts before Plan/Todo.
- Requires message ids, deduplication, compaction boundary handling, and list
  projection changes.

Good later direction, but too large for the first Phase A-PRE slice.

### Option D: Remove Session Cache Entirely

Delete GUI session history cache and always read backend.

Pros:

- Strongest GUI consistency.
- Simpler mental model.

Cons:

- Worse perceived responsiveness.
- More backend dependency.
- May hurt offline/error UX.

Acceptable only if Option B's cache fallback becomes too complex.

## Recommendation

Use **Option B: Backend Durability Plus GUI Reconciliation**.

This keeps the GUI responsive while making backend history authoritative and
durable enough for Plan/Todo. It is the minimum fix that addresses both backend
message loss and frontend stale-cache divergence.

## Implementation Phases

### A-PRE-0: Confirmed Call-Site Audit

Deliverables:

- Confirm the exact code sites listed in the Code Map.
- Record which subagent research findings are accepted, downgraded, or rejected.
- Keep the raw research reports as evidence, not as direct implementation scope.

Acceptance:

- The implementation issue/PR references this corrected Phase A-PRE document.
- P0 scope is clear and not inflated by unverified findings.
- No behavior changes yet.

### A-PRE-1: Backend Durable Inbound Save

Deliverables:

- Persist inbound user message immediately after `get_or_create`.
- Avoid duplicate user-message append at final `save_turn`.
- Add tests for cancellation/error-before-final-response preserving user input.
- Surface immediate persistence failure.

Acceptance:

- User message remains in backend session history after cancellation.
- User message remains after provider/stream error before final assistant output.
- No duplicated user message after successful turn.

### A-PRE-2: Atomic Save And Load Failure Safety

Deliverables:

- `SessionManager::save` uses temp-file plus rename or equivalent atomic write.
- `SessionManager::load` distinguishes missing file from read/parse failure.
- Parse/read failures are logged or returned visibly.
- Unreadable/corrupt existing files are not overwritten by empty sessions.

Acceptance:

- Existing JSONL file survives a failed save attempt.
- Read failure does not create and persist an empty replacement session.
- Malformed JSONL lines are reported, not silently discarded without trace.

### A-PRE-3: Consolidation Ordering

Deliverables:

- Persist raw turn messages before consolidation side effects.
- Ensure `last_consolidated` advancement is not durable unless the matching
  session state is durable.
- Add regression coverage around consolidation save ordering.

Acceptance:

- Crash/save failure cannot leave memory/HISTORY ahead of session JSONL without
  a visible dirty/error state.
- Repeated consolidation of the same segment is avoided.

### A-PRE-4: Backend-First GUI Loading

Deliverables:

- `loadSession()` fetches backend first.
- Cache is refreshed only from backend canonical history.
- Cache fallback is used only when backend request fails.
- Fallback state is visible in UI.

Acceptance:

- Backend-up path ignores stale 30-minute cache.
- Backend-down path can still show cached session with a stale warning.
- Backend "session not found" does not silently resurrect stale cache.

### A-PRE-5: GUI Cache Invalidation And Optimistic Reconciliation

Deliverables:

- Add or reuse cache invalidation helper.
- Invalidate session cache on send completion/failure/delete/reset/new
  session/session switch.
- Pending user/assistant messages are marked local-only.
- Successful turn replaces local render state with backend canonical history.
- Failed turn removes empty assistant placeholder or marks it failed.
- Local-only pending messages are not persisted as canonical cache history.

Acceptance:

- Deleted/reset sessions do not reappear from local cache.
- Switching sessions cannot resurrect stale messages.
- A failed send does not leave a permanent empty assistant message.
- Refresh after success matches backend JSONL/session history.

### A-PRE-6: Regression Tests

Deliverables:

- Backend tests for early user-message persistence.
- Backend tests for atomic save/read failure behavior.
- Backend tests for consolidation ordering where practical.
- Frontend unit tests for cache priority and invalidation.
- Component tests for send failure placeholder behavior.
- Tauri command mock tests if existing test harness supports them.

Acceptance:

- Tests cover backend-first load, backend-fallback load, reset/delete cache
  invalidation, and failed send cleanup.

## Test Matrix

### Early User Message Persistence

Setup:

- Start a turn.
- Cancel or force an error before final assistant output.

Expected:

- Backend session history contains the inbound user message.
- The final assistant message may be absent or failed, but the user input is not
  lost.

### Atomic Save Failure

Setup:

- Create a valid session JSONL.
- Simulate save failure during write.

Expected:

- Previous session file remains readable.
- New partial data is not installed as the canonical session file.

### Load Failure Does Not Overwrite

Setup:

- Create a session file that exists but cannot be parsed/read.
- Call `get_or_create` or the equivalent load path.

Expected:

- The system reports the load failure.
- It does not silently create and save an empty session over the old file.

### Consolidation Ordering

Setup:

- Trigger a turn that reaches consolidation threshold.
- Simulate session save failure around consolidation.

Expected:

- Memory/HISTORY and session `last_consolidated` cannot silently diverge.

### Backend History Wins

Setup:

- Put stale messages in `agent-diva-session-cache:<id>`.
- Mock backend to return newer messages.

Expected:

- GUI renders backend messages.
- Cache is overwritten/refreshed.

### Backend Fallback

Setup:

- Put cached messages in localStorage.
- Mock backend history fetch failure.

Expected:

- GUI renders cached messages.
- UI indicates local cache fallback / possibly stale.

### Send Success Reconciliation

Setup:

- User sends message.
- GUI creates optimistic pending state.
- Backend turn succeeds with canonical history.

Expected:

- GUI messages equal backend canonical history.
- Pending placeholder is gone.
- Cache reflects backend canonical history.

### Send Failure Cleanup

Setup:

- User sends message.
- Backend turn fails before assistant content is persisted.

Expected:

- Empty assistant placeholder is removed or marked failed.
- It is not saved as canonical cache.
- User sees an error state.

### Reset/Delete/Switch

Setup:

- Populate session cache.
- Perform reset, delete, or switch.

Expected:

- Matching `agent-diva-session-cache:*` key is removed.
- Old messages do not reappear on reload.

## Validation Commands

Use the repo's Rust validation path for backend changes:

```powershell
just fmt-check
just check
just test
```

For targeted backend iteration, likely useful direct tests:

```powershell
cargo test -p agent-diva-core session
cargo test -p agent-diva-agent
```

Use the repo's existing frontend validation path for GUI changes. Likely
commands include:

```powershell
cd agent-diva-gui
npm run test -- <relevant test files>
npm run build
```

If the change touches Tauri Rust commands:

```powershell
cargo test -p agent-diva-gui
```

Final validation should also include a manual GUI smoke test:

1. Open GUI.
2. Load a session with stale local cache and newer backend history.
3. Confirm backend history wins.
4. Send a message and interrupt/fail the turn if possible.
5. Confirm backend session still contains the user input.
6. Confirm no permanent empty assistant placeholder remains.
7. Reset/delete/switch session and confirm old messages do not return.

## Risks And Controls

| Risk | Control |
| --- | --- |
| User message is saved twice | Split inbound-message persistence from assistant/tool finalization; add duplicate regression test |
| Immediate save adds latency | Keep save narrow; consider batching later only after durability semantics are correct |
| Atomic write behavior differs across platforms | Use same-directory temp file plus rename and test on Windows |
| Load error handling breaks old corrupt sessions | Preserve file and surface error instead of overwriting; add recovery notes |
| Consolidation order changes memory behavior | Persist raw turn first and test threshold behavior |
| Backend fetch latency hurts UI | Show loading state; cache fallback only on failure |
| Cache fallback hides backend failure | Display explicit stale-cache warning |
| Optimistic message merge duplicates messages | Prefer canonical replacement after backend success |
| Reset/delete races with cache writes | Invalidate after mutation and guard stale writes by session id |
| Tests overfit implementation details | Test behavior: backend wins, fallback only on error, placeholder cleanup |
| Plan/Todo builds on stale GUI state | Make this phase a gate before Phase A |

## Acceptance Criteria

- Inbound user message is persisted before LLM/tool execution.
- Cancel or provider/stream failure before final response does not lose user
  input.
- Session save is atomic enough to preserve the old JSONL on failed write.
- Read/parse failures do not silently overwrite old session files with empty
  sessions.
- Raw turn state is durably saved before consolidation side effects become
  authoritative.
- `loadSession()` defaults to backend history.
- Local session cache is used only when backend history is unavailable.
- Backend success refreshes or replaces local cache.
- Send success reconciles GUI state with backend canonical history.
- Send failure does not leave a permanent empty assistant placeholder.
- Reset/delete/switch/new session invalidates matching session cache.
- Reloaded GUI history matches backend JSONL/session history when backend is
  available.
- The implementation has focused tests for cache priority, invalidation, and
  optimistic failure behavior.
