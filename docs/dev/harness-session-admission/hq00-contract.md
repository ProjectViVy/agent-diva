# HQ-00 Session Admission Contract

Status: frozen on 2026-08-29 for `HARNESS-SESSION-ADMISSION-BOUNDED-QUEUE`.

## Scope

HQ-00 freezes the ownership, lifecycle, configuration, and observable outcome contract for a future
per-session bounded admission queue. It adds characterization coverage only. It does not add a queue,
change runtime behavior, alter wire formats, or move the MessageBus authority.

The current baseline is intentionally preserved and characterized:

- `AgentLoop::run(&mut self)` consumes inbound messages globally and serially; a blocked turn prevents a
  second session from reaching the provider.
- The canonical session key is `channel:chat_id`. The explicit `session_id` argument accepted by direct
  entry points is compatibility-only and must not create a second identity authority.
- A `StopSession` command can be observed while a provider stream is pending and cancels the running turn.
- MessageBus remains transport. Session scheduling belongs behind the Agent Loop admission seam.

## Frozen configuration

The future additive configuration location is `agents.defaults.session_admission`, represented by a
`SessionAdmissionConfig`-style type with serde defaults. HQ-03 will wire configuration and compatibility;
HQ-00 adds no public configuration fields.

| Setting | Default | Meaning |
| --- | ---: | --- |
| `max_queue_depth` | `2` | Number of waiting turns per session; the running turn is excluded. |
| `wait_timeout` | `30s` | Maximum time from enqueue until lease acquisition. |
| `idle_ttl` | `10m` | Time an empty, non-running session slot may remain before eviction. |

When a session already has one running turn and two waiting turns, the newest arrival is rejected. The
existing queued turns retain FIFO order.

## Ownership model

| Layer | Owned state and responsibility |
| --- | --- |
| Dispatcher | Inbound/control receivers, canonical-key routing, session-worker registry, supervision, idle reap, and shutdown coordination. |
| Process-shared | MessageBus, provider, workspace/persona/config snapshots, FileManager, MemoryProvider, SubagentManager, and shareable admission/circuit services. |
| Session worker | One FIFO mailbox, session persistence/cache entry, context cache entry, cancellation generation/token, ACTMEM timer, active/deferred tool state, and pending checkpoints for one canonical session key. |
| Turn-local | Request/trace identity, InboundMessage, mode/model/approval snapshot, ToolRegistry view, planning snapshot, admission outcome, and event sink. |

`apply_approval_policy_from_metadata` currently mutates loop-global `ToolConfig`. HQ-02 must make the
approval/tool surface turn-local (or immutable per-turn) before different session workers run concurrently;
otherwise policy can bleed across sessions. No provider/tool/BML await may hold a dispatcher or admission
kernel lock.

The implementation must use a dispatcher plus per-session worker/lease model. A global mutex, a copied
`AgentLoop` per request, or a MessageBus rewrite does not satisfy the contract and requires architecture
re-review before HQ-02 proceeds.

## Lifecycle and control semantics

The per-session state machine is `idle -> running -> idle`, with zero to two FIFO waiters while running.
Admission expiry removes the expired waiter before the next candidate is selected. FIFO order is determined
at successful enqueue, not task wake-up time.

- `StopSession`: cancel only the running turn; queued turns remain and the oldest proceeds after the running
  lease reaches quiescence.
- `ResetSession` and `DeleteSession`: cancel the running turn and all queued turns. Persistence reset/delete
  and slot cleanup occur only after the worker is quiescent.
- Queue-full: reject the newest arrival before provider, tool, session-history, or BML side effects.
- Wait timeout: remove and reject only the expired waiter before those same side effects.
- Idle eviction: allowed only when there is no running lease and no waiter.
- Runtime shutdown: stop accepting new work and drain already accepted turns; shutdown is not a logical
  Reset/Delete.
- Worker failure: reject remaining accepted waiters deterministically, remove the dead slot, and allow a later
  request to create a fresh worker.

## Stable future outcomes and correlation

HQ-03 must expose stable machine-readable codes without parsing prose:

- `session_queue_full`
- `session_queue_wait_timeout`
- `session_turn_cancelled`
- `session_reset`
- `session_worker_unavailable`
- `session_slot_evicted`

Observable events must carry canonical session key, request/trace identity, queue depth, admission phase, and
wait latency. The future public type shape may use `SessionAdmissionCode`, `SessionAdmissionPhase`, and
`SessionAdmissionSnapshot`; names can change only before HQ-03 publishes them.

`AgentBusEvent` currently correlates primarily by channel/chat. Manager streaming can therefore cross-talk if
two requests target the same session concurrently. Real concurrent wiring must not ship until an optional,
backward-compatible request/trace field is projected through AgentBusEvent and Manager/CLI/GUI consumers.

## Phase gates

- HQ-01 implements only the deterministic, clock-injectable admission kernel and its race tests.
- HQ-02 introduces dispatcher/session-worker ownership and converges Bus/direct entry points on one seam.
- HQ-03 publishes configuration, typed outcomes, runtime-control mapping, and request correlation.
- HQ-04/05 prove cross-entry behavior, fault handling, rollback, and release readiness.

Stop and return to architecture review if implementation requires a global lock across awaits, independent
copies of mutable AgentLoop state, a MessageBus authority rewrite, or changes to PlanMode, Sandbox, Approval,
session history, BML, Persona, or Evolution authority.
