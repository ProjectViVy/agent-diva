# agent-diva GUI Frontend Cache & Optimistic UI Depth Audit

**Date:** 2026-06-01
**Scope:** `agent-diva-gui/src/App.vue`, `ChatView.vue`, `localStorageAgentDiva.ts`
**Author:** subagent (deep audit)

---

## Executive Summary

The GUI frontend implements a **cache-first read policy with no invalidation on writes**. The localStorage session cache is written exactly once (on first load from backend), never refreshed after user interactions, and blindly trusted for 30 minutes. Combined with optimistic UI push that never cleans up on failure, this creates multiple data-loss and stale-data surfaces.

**5 P0 (critical) bugs, 4 P1 (high) bugs, 3 P2 (medium) bugs found.**

---

## 1. P0 Bugs — Critical

### BUG-1: Cache-First with Zero Invalidation → Permanent Stale Reads

**Severity:** P0
**File:** `App.vue`
**Lines:** 781-799 (loadSession), 462-478 (readSessionFromCache), 481-494 (writeSessionToCache)

**Description:**

`loadSession()` always reads from localStorage cache first. If a cache hit occurs (data present and cachedAt within 30 min TTL), the backend is **never called**:

```typescript
// App.vue:786-799
let sessionHistory = readSessionFromCache(sessionKey);   // <-- cache-first
if (!sessionHistory && chatId && chatId !== sessionKey) {
  sessionHistory = readSessionFromCache(chatId);         // <-- fallback cache keys
}
if (!sessionHistory) {                                   // <-- backend ONLY on double-miss
  sessionHistory = await invoke("get_session_history", ...);
  writeSessionToCache(sessionHistory);                   // <-- ONLY cache write site
}
```

`writeSessionToCache()` is called **exclusively** inside `loadSession()` on line 797. It is never called from `sendMessage()`, from streaming completion handlers (`agent-response-complete`), from `stopMessage()`, from `deleteSession()`, or from any timer/background task.

**Reproduction:**

1. App starts → `refreshSessions()` → `restoreLatestGuiChatOnStartup()` → `loadSession("gui:chat-123")` → cache miss → backend fetch → `writeSessionToCache(data_v1)`.
2. User types "Hello" and sends → `sendMessage()` pushes user msg + empty assistant placeholder → backend responds via streaming events → `messages.value` now has data_v1 + "Hello" exchange.
3. User clicks sidebar to switch to "gui:chat-456" → `loadSession("gui:chat-456")` loads.
4. User clicks back to "gui:chat-123" → `loadSession("gui:chat-123")` → `readSessionFromCache` **hits** → returns `data_v1` (stale, missing the "Hello" exchange) → `messages.value` is **overwritten** with stale data.
5. All messages from step 2 are LOST from the UI.

**Root Cause:** Cache is never invalidated or refreshed after any mutation. There is no stale-while-revalidate, no write-through, no TTL-based background refresh.

---

### BUG-2: sendMessage Success Does Not Update Cache

**Severity:** P0
**File:** `App.vue`
**Lines:** 576-655 (sendMessage), 1149-1164 (agent-response-complete handler)

**Description:**

After `sendMessage()` successfully sends a message and receives a streaming response, the localStorage session cache is **never updated**. This compounds BUG-1: even the first cache entry is stale because it was captured before the user sent any message in the current session.

The complete flow:
- `sendMessage()` → invokes backend → streaming events update `messages.value` → `agent-response-complete` fires → `isStreaming = false`
- At NO point is `writeSessionToCache()` called with the updated messages.
- The cache still holds the pre-send state forever (until TTL expires or manual cache clear).

---

### BUG-3: Optimistic User Message Never Removed on Error

**Severity:** P0
**File:** `App.vue`
**Lines:** 590-654

**Description:**

The user message is pushed to `messages.value` at line 596 **before** the backend call. On error (lines 635-654), only the streaming assistant placeholder is popped:

```typescript
// Line 596
messages.value.push(userMsg);   // pushed BEFORE invoke

// ...

} catch (error) {
  // Line 640-644: only removes assistant placeholder
  if (messages.value.length > 0) {
    const lastMsg = messages.value[messages.value.length - 1];
    if (lastMsg.role === 'agent' && lastMsg.isStreaming) {
      messages.value.pop();     // <-- pops assistant, NOT user msg!
    }
  }
  // Line 647-651: pushes error system message
  messages.value.push({ role: 'system', content: `[Error] ${error}`, ... });
}
```

The user's message (`role: 'user'`) **persists in the message list** even though it was never successfully sent to the backend. The UI shows: [user message] → [error system message], which misleads the user into thinking their message was delivered.

---

### BUG-4: Empty Streaming Placeholder Stuck Forever on Streaming Gap

**Severity:** P0
**File:** `App.vue`
**Lines:** 511-525 (closeStreamingPlaceholder), 600, 605-611 (placeholder creation)

**Description:**

`sendMessage()` pushes an empty assistant placeholder with `isStreaming: true` (lines 605-611). The only cleanup path for this placeholder when content stays empty is:

1. `closeStreamingPlaceholder(true)` at line 600 — called at the start of the **next** `sendMessage()`, not self-healing.
2. Error handler at lines 640-644 — pops the placeholder if it's the last message with `isStreaming`.
3. `agent-error` handler at lines 1295-1298 — pops the placeholder.

If the backend `invoke("send_message")` succeeds but **no streaming events fire** (e.g., backend starts processing but event bus fails, `agent-response-delta`/`agent-response-complete` never arrive), the placeholder stays permanently with the bouncing dots animation (`isStreaming: true`, `content: ''`).

The only way to clear it is to send another message (which calls `closeStreamingPlaceholder(true)`) or receive an `agent-error` event. There is no timeout-based cleanup.

---

### BUG-5: activeStreamRequestId Overwrite Orphans Previous Stream

**Severity:** P0
**File:** `App.vue`
**Lines:** 601-602, 1123-1131 (agent-response-delta handler)

**Description:**

Each `sendMessage()` generates a new `streamRequestId` and overwrites `activeStreamRequestId.value`:

```typescript
// Line 601-602
const streamRequestId = generateStreamRequestId();
activeStreamRequestId.value = streamRequestId;  // <-- overwrites without checking old
```

All streaming event handlers check `event.payload.request_id !== activeStreamRequestId.value` and silently return if mismatched:

```typescript
// Lines 1124-1126
if (event.payload.request_id !== activeStreamRequestId.value) {
  return;  // <-- silently drops events for old stream
}
```

If the user rapidly sends two messages (or the first send is still streaming and the user sends another), the first stream's events are permanently ignored. The first placeholder may remain with partial content and `isStreaming: true`, never transitioning to `isStreaming: false`.

`closeStreamingPlaceholder(true)` at line 600 only handles the **last** streaming placeholder (it scans from `messages.value.length - 1` backward and stops at the first match). It does NOT clean up the previous stream's placeholder.

---

## 2. P1 Bugs — High Priority

### BUG-6: Tool Event Handlers Create Redundant Agent Placeholders

**Severity:** P1
**File:** `App.vue`
**Lines:** 1174-1207 (agent-tool-start), 1209-1264 (agent-tool-end)

**Description:**

Both `agent-tool-start` and `agent-tool-end` handlers push new `agent` placeholders in addition to the tool messages:

- `agent-tool-start` (lines 1199-1206): pushes a new `{role: 'agent', content: '', isStreaming: true}` placeholder.
- `agent-tool-end` (lines 1257-1263): pushes **another** `{role: 'agent', content: '', isStreaming: true}` placeholder.

This means a single tool call sequence produces:
1. Original assistant placeholder (from sendMessage)
2. Tool "running" message
3. New assistant placeholder (from tool-start)
4. Tool "success/error" message (updated)
5. Another assistant placeholder (from tool-end)

If `agent-response-delta` events follow, they append to the **last** streaming agent message. But if tool-end fires after the final response stream completes, the extra placeholder becomes orphaned.

```
sendMessage  → push [assistant-streaming]
tool-start   → pop maybe [assistant], push [tool-running], push [assistant-streaming]  
tool-end     → pop maybe [assistant], find tool msg, push [assistant-streaming]
...streaming events append to last [assistant-streaming]
agent-complete → set last isStreaming=false
// Extra tool-end placeholder may remain if timing is wrong
```

---

### BUG-7: stopMessage Does Not Clean Up Placeholder or Cache

**Severity:** P1
**File:** `App.vue`
**Lines:** 657-699

**Description:**

`stopMessage()` marks the last streaming message as `isStreaming = false` (lines 682-684) and pushes a system message. However:

1. It does **not** call `writeSessionToCache()` to persist the partially-complete response.
2. It does **not** remove any empty/partial placeholders that may have accumulated from streaming.
3. It does **not** clear `activeStreamRequestId` — if the backend returns an error after stop, the `agent-error` handler may process it with a stale `requestId` check.

---

### BUG-8: deleteSession on Backend-Failure Still Removes UI Session

**Severity:** P1
**File:** `App.vue`
**Lines:** 852-891

**Description:**

If `invoke('delete_session')` fails (line 859-861), the function still:
- Removes localStorage cache keys (lines 869-877)
- Adds to `locallyDeletedSessionKeys` (line 878)
- Removes from `sessions.value` (line 879)

The session disappears from the UI but still exists on the backend. A subsequent `refreshSessions()` (called at line 880) may re-list it, but `locallyDeletedSessionKeys` filters it out in `refreshSessions()` (line 740), so it stays hidden even though it was never deleted.

The only indication to the user is a system message "Delete failed on backend; removed locally for this run." (lines 884-889), which is easy to miss.

---

### BUG-9: 30-Minute TTL is Too Long for Multi-Instance Safety

**Severity:** P1
**File:** `App.vue`
**Lines:** 133 (`SESSION_CACHE_TTL_MS = 30 * 60 * 1000`)

**Description:**

During the 30-minute cache window, any session mutation performed by:
- The CLI tool
- A cron job
- Another GUI window (same Tauri app instance)
- External programmatic access

is **completely invisible** to the GUI. The user sees stale data with zero indication that the data may be out of date. There is no "last updated" timestamp displayed, no background refresh, and no manual "refresh session" button exposed to the user.

---

## 3. P2 Bugs — Medium Priority

### BUG-10: clearMessages Does Not Clean Old Session Cache

**Severity:** P2
**File:** `App.vue`
**Lines:** 701-719

**Description:**

When the user clicks "New Session" (triggers `clearMessages` → emits `clear` from ChatView → `clearMessages` in App.vue):

```typescript
function clearMessages() {
  currentChatId.value = generateChatId();        // <-- new random ID
  currentSessionKey.value = `gui:${currentChatId.value}`;
  // ...
  messages.value = [{ role: 'agent', content: t('app.cleared'), ... }];
}
```

The old session's localStorage cache entry is **not removed**. These orphaned cache entries accumulate indefinitely and can only be cleared by the global cache clear button in General Settings. Over time this wastes storage.

---

### BUG-11: No Cross-Window Cache Invalidation

**Severity:** P2
**File:** `App.vue`, `localStorageAgentDiva.ts`

**Description:**

The localStorage session cache is global across all Tauri windows (shared WebView storage). If window A deletes a session, window B's cache still holds it and will serve stale data via `readSessionFromCache`. There is no `storage` event listener, BroadcastChannel, or IPC notification for cache invalidation across windows.

---

### BUG-12: refreshSessions Does Not Invalidate Session Caches

**Severity:** P2
**File:** `App.vue`
**Lines:** 721-746

**Description:**

`refreshSessions()` fetches the session list from the backend but only uses it to populate the sidebar session list (`sessions.value`). It does not:
- Cross-check cached session data against backend
- Invalidate stale caches
- Trigger a re-fetch of the currently active session's data

This could be a natural cache-invalidation point but is completely disconnected from the caching layer.

---

## 4. Architecture Diagram

```
                           ┌──────────────────────────────┐
                           │        User Actions           │
                           │  Send / Stop / Switch / Delete│
                           └──────────┬───────────────────┘
                                      │
                         ┌────────────┼────────────┐
                         ▼            ▼            ▼
                   ┌──────────┐ ┌──────────┐ ┌──────────┐
                   │sendMessage│ │stopMessage│ │loadSession│
                   │ Lines:576 │ │ Lines:657│ │ Lines:781│
                   └─────┬─────┘ └─────┬────┘ └─────┬─────┘
                         │             │             │
              ┌──────────┼──┐          │    ┌────────┼────────┐
              ▼          ▼  ▼          │    ▼        ▼        ▼
         ┌─────────┐ ┌──────────┐     │ ┌──────────────────────────┐
         │messages │ │ optimistic│     │ │ readSessionFromCache     │
         │.value   │ │ push user │     │ │ (CACHE-FIRST, TTL=30min)│
         │(Vue)    │ │ + empty   │     │ │ Lines: 462-478          │
         └────┬────┘ │ assistant │     │ └───────────┬──────────────┘
              │      └─────┬─────┘     │             │
              │            │           │        ┌────┴────┐
              │  ┌─────────▼─────────┐ │   HIT? │  CACHE  │──YES──► return stale
              │  │ invoke("send_msg")│ │        │  HIT?   │          (BUG-1, BUG-2)
              │  └─────────┬─────────┘ │        └────┬────┘
              │            │           │             │ NO
              │       ┌────┴────┐      │             ▼
              │       │ SUCCESS?│      │    ┌──────────────────────┐
              │       └────┬────┘      │    │ invoke("get_session  │
              │       YES  │  NO       │    │ _history") (Backend) │
              │            │           │    └──────────┬───────────┘
              │    ┌───────┴───────┐   │               │
              │    │ NEVER updates │   │    ┌──────────▼───────────┐
              │    │ localStorage  │   │    │ writeSessionToCache  │
              │    │ cache! (BUG-2)│   │    │ (ONLY cache write!)  │
              │    └───────────────┘   │    │ Lines: 481-494       │
              │                        │    └──────────────────────┘
              │            ┌───────────┴───────────┐
              │            │ ERROR PATH (BUG-3):    │
              │            │ pops assistant only,   │
              │            │ user msg stays forever  │
              │            └───────────────────────┘
              │
              ▼
    ┌─────────────────────┐
    │ Streaming Events     │
    │ agent-response-delta │──► check activeStreamRequestId
    │ agent-reasoning-delta│    (BUG-5: old stream dropped)
    │ agent-response-complete──► isStreaming=false, NO cache write
    │ agent-error          │──► pop placeholder (partial cleanup)
    │ agent-tool-start     │──► push tool+agent msg (BUG-6)
    │ agent-tool-end       │──► push agent msg (BUG-6)
    └─────────────────────┘
```

**Legend:**
- Solid arrows = code execution flow
- Dotted arrows = missing/incomplete flows
- Bold boxes = critical bug sites
- localStorage cache is written at EXACTLY ONE point (loadSession cache miss)
- localStorage cache is read at EXACTLY TWO points (both in loadSession)
- There is ZERO cache invalidation anywhere in the codebase

---

## 5. Cache Key Mapping

| Operation | Input Key | Cache Key(s) Written/Read |
|-----------|----------|--------------------------|
| writeSessionToCache | `session.key` from backend (e.g. "gui:chat-123") | `agent-diva-session-cache:gui:chat-123` |
| readSessionFromCache("gui:chat-123") | sessionKey = "gui:chat-123" | `agent-diva-session-cache:gui:chat-123` |
| readSessionFromCache("chat-123") | chatId (no prefix) | `agent-diva-session-cache:gui:chat-123` + `agent-diva-session-cache:chat-123` |
| deleteSession | sessionKey + chatId | all `getSessionCacheKeys(sessionKey)` + `getSessionCacheKeys(chatId)` |

**No mismatch risk** — the double-read in loadSession (lines 786-789 covering both sessionKey and chatId) and the dual-key generation in getSessionCacheKeys handle the variance.

---

## 6. Fix Recommendations

| Bug | Fix |
|-----|-----|
| **BUG-1/2** Cache-First + No Invalidation | Add `writeSessionToCache(messages)` call after `agent-response-complete`. Change `loadSession` to stale-while-revalidate: serve cache immediately, then async-refresh from backend. |
| **BUG-3** User msg lingers on error | Use a temporary message (not pushed to `messages.value`) or pop the user message in the catch block alongside the assistant. |
| **BUG-4** Stuck placeholder | Add a 30-second timeout that auto-clears `isStreaming` if content is still empty. |
| **BUG-5** activeStreamRequestId race | Before overwriting, call `closeStreamingPlaceholder(true)` on the old stream's placeholder (scan all, not just the last). |
| **BUG-6** Redundant placeholders | Don't push new agent placeholder in `agent-tool-end` — the `agent-tool-start` already handles it. Only push if no existing streaming agent placeholder exists. |
| **BUG-7** stopMessage no cache write | Call `writeSessionToCache` with current messages snapshot before marking `isStreaming=false`. |
| **BUG-8** deleteSession UI/backend desync | Roll back UI changes if backend delete fails; don't add to `locallyDeletedSessionKeys`. |
| **BUG-9** TTL too long | Reduce to 5 minutes OR implement stale-while-revalidate with a 30-second background refresh. Show a "last synced" indicator. |
| **BUG-10** Orphaned caches | Call `writeSessionToCache` with final state when creating a new session, or remove old cache key in `clearMessages`. |
| **BUG-11** Cross-window | Add `window.addEventListener('storage', ...)` to detect external localStorage changes and invalidate accordingly. |
| **BUG-12** refreshSessions gap | After `refreshSessions()`, check if current session's backend `updated_at` is newer than `cachedAt` and trigger re-fetch. |

---

*End of audit.*
