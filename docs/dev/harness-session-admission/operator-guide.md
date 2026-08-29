# Session Admission Operator Guide

Status: production contract completed by `HARNESS-SESSION-ADMISSION-BOUNDED-QUEUE` on
2026-08-30.

## Purpose and boundary

Session admission serializes turns that share the canonical `channel:chat_id` session key while
allowing different sessions to execute concurrently. The queue sits behind Agent Loop admission;
`MessageBus` remains transport and does not own scheduling. A rejected request does not reach the
provider, tools, session history, or BML write surfaces.

## Configuration

The additive configuration object lives in `~/.agent-diva/config.json` (or the file selected by
`--config`):

```json
{
  "agents": {
    "defaults": {
      "session_admission": {
        "max_queue_depth": 2,
        "wait_timeout": 30,
        "idle_ttl": 600
      }
    }
  }
}
```

| Field | Default | Valid values | Meaning |
| --- | ---: | --- | --- |
| `max_queue_depth` | `2` | integer >= 0 | Waiting turns per session; the running turn is excluded. `0` rejects every waiter. |
| `wait_timeout` | `30` | integer > 0 | Maximum queue wait in seconds. |
| `idle_ttl` | `600` | integer > 0 | Seconds before a fully idle worker slot may be evicted. |

Run `agent-diva --config <path> config validate` after editing. Restart the process that owns the
Agent Loop (CLI gateway or embedded desktop Gateway) to apply the new limits.

## Migration and compatibility

No manual migration is required. Existing configuration files that omit `session_admission` receive
the defaults above through serde defaults. Saving configuration may begin persisting the additive
object. No session-history, database, BML, Persona, or workspace data is rewritten.

The public observation fields are additive. Consumers that do not understand admission events may
ignore them and continue consuming ordinary delta/final/error events.

## Runtime outcomes

Admission observations preserve `session_key`, `request_id`, `trace_id`, `phase`, `queue_depth`, and
`wait_latency_ms`. Stable terminal codes are:

| Code | Operator meaning |
| --- | --- |
| `session_queue_full` | The configured waiter capacity was already occupied; the newest request was rejected. |
| `session_queue_wait_timeout` | The request expired before acquiring the session lease. |
| `session_turn_cancelled` | The running request was cancelled by runtime control. |
| `session_reset` | Reset/Delete cancelled accepted work for the session before cleanup. |
| `session_worker_unavailable` | The owning worker failed; affected requests were drained and a later request may recreate it. |
| `session_slot_evicted` | An idle slot was removed; this is lifecycle observation, not loss of an active turn. |

Queued and running observations are nonterminal. The desktop changes state only when both the stream
wrapper and observation belong to its active request.

## Runtime control

- Stop cancels only a matching running request. A matching queued request returns
  `queued_preserved` and retains FIFO position.
- Reset and Delete cancel the running request and all queued requests, then perform durable cleanup
  after the worker becomes quiescent.
- Shutdown stops new admission and drains already accepted work; it is not Reset/Delete.
- A worker panic resolves affected requests as `session_worker_unavailable`, removes only the failed
  worker generation, and permits clean reconstruction on the next request.

## Capacity guidance

- Keep the default depth of two unless measured queue pressure justifies a change.
- Use depth zero for strict fail-fast operation, not to disable serialization.
- Raise `wait_timeout` only when provider latency is expected and clients can tolerate the added
  response time.
- `idle_ttl` controls worker-registry retention only; eviction never removes running or queued work.

## Rollback

There is no runtime feature flag that restores the old global-serial path. For emergency whole-Epic
rollback, stop accepting traffic, let accepted turns quiesce, and deploy the pre-implementation
baseline `8b901d4f`. Preserve `config.json`; the older binary ignores the additive
`session_admission` object. No data rollback is needed.

For a source revert on the current line, revert the implementation commits in reverse dependency
order and rerun the full release gates:

1. `0226571e`, `5176ac18`, `2e3553fb`
2. `011db1f3`
3. `b53c619f`, `9302f8e7`
4. `5ff59b5c`

Do not partially revert the admission kernel while retaining runtime wiring.
