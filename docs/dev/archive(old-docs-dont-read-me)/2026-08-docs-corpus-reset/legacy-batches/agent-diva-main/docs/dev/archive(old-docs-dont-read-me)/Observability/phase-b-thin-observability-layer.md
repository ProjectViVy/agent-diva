# Phase B: Thin Observability Layer

## Background And Goal

After Phase A (`Plan + TodoList`), the next enhancement should be a thin
observability layer. The goal is not to build a full observability platform, but
to make agent-diva easier to debug, audit, and support.

The guiding principle is:

```text
Keep the feature thin.
Make logs accurate, durable, correlated, redacted, and exportable.
```

Good logs have already helped debug agent-diva issues. This phase should turn
that practical experience into a small shared trace/log foundation that works
across the agent runtime, tools, planning events, gateway, and future GUI debug
settings.

## Non Goals

- No full replay engine.
- No large trace dashboard.
- No OpenTelemetry requirement in the MVP.
- No Prometheus/metrics stack.
- No provider full-payload archive by default.
- No multi-worker Kanban observability.
- No GUI log analyzer in the MVP.
- No large taxonomy of many log files in the first slice.

## Target Capability

For any meaningful runtime issue, an operator should be able to answer:

- What user/channel/cron event started this?
- Which agent session handled it?
- Which LLM call happened?
- Which tools ran?
- Which gateway inbound/outbound events happened?
- Which Plan/Todo events were involved?
- Where did it fail?
- Can I export enough redacted context for a developer to debug it?

The minimum target trace path is:

```text
gateway/user input
  -> trace_id
  -> agent loop
  -> provider request/response summary
  -> tool calls
  -> plan/todo events
  -> gateway/user output
```

## MVP Pillars

### 1. Unified Trace ID

Each external trigger should get a `trace_id`:

- user message
- gateway inbound event
- cron trigger
- manager/API-triggered task
- future Plan/Todo execution request

The `trace_id` should flow through:

- `MessageBus`
- `AgentLoop`
- provider calls
- tool calls
- Plan/Todo events
- gateway inbound/outbound
- manager debug APIs

`trace_id` is the main correlation key. Without it, additional logs are much
less useful.

### 2. Structured JSONL Runtime Log

Start with one structured log file per day:

```text
.agent-diva/logs/runtime-YYYY-MM-DD.jsonl
```

The first version should avoid splitting logs into many files. One file keeps
inspection and debug bundle export simple.

Suggested event shape:

```json
{
  "ts": "2026-06-01T12:00:00Z",
  "level": "info",
  "trace_id": "tr_...",
  "session_id": "slack:C123",
  "channel": "slack",
  "component": "agent_loop",
  "event": "tool_call_completed",
  "summary": "read_file completed",
  "metadata": {
    "tool": "read_file",
    "duration_ms": 32,
    "status": "ok"
  }
}
```

Required top-level fields:

- `ts`
- `level`
- `trace_id`
- `session_id`
- `channel`
- `component`
- `event`
- `summary`
- `metadata`

`metadata` must stay structured and redacted.

### 3. Minimal Event Set

The MVP should log only high-signal events:

- `gateway_inbound`
- `gateway_outbound`
- `gateway_error`
- `message_received`
- `llm_request_started`
- `llm_response_completed`
- `llm_response_failed`
- `tool_call_started`
- `tool_call_completed`
- `tool_call_failed`
- `plan_event`
- `todo_event`
- `verification_result`
- `runtime_cancelled`

More event types can be added later, but this set is enough to trace most
end-to-end failures.

### 4. Redaction And Retention

Defaults must favor safety:

- Do not record full provider payloads by default.
- Do not record full tool outputs by default.
- Truncate large outputs.
- Redact tokens, API keys, cookies, authorization headers, webhook secrets, and
  common private key formats.
- Keep default retention short, such as 7 or 14 days.
- Allow users to clear logs.

Redaction should happen before writing structured logs, not only during export.

### 5. Debug Bundle

Provide a lightweight debug bundle path before building a full GUI log viewer.

The bundle should include:

- recent JSONL logs for a selected time window
- current redacted config summary
- active Plan/Todo state if present
- recent gateway errors
- runtime version/build metadata if available

The bundle should not include secrets or full provider payloads unless the user
explicitly enables a more verbose debug mode.

## GUI Debug Settings Scope

The GUI should initially expose settings and actions, not a complex analysis
view.

Suggested settings:

- enable structured runtime logs
- log level
- retention days
- record tool output summaries
- record provider payloads: off by default
- record gateway raw messages: off by default
- export debug bundle
- clear logs

The GUI can later grow a trace browser, but the first phase should prioritize
correct logging and safe export.

## Gateway Logging Requirements

Gateway logging needs special attention because it is the long-running entry
point and often the source of hard-to-reproduce issues.

Log at least:

- channel inbound message received
- channel outbound message sent
- outbound delivery failure
- adapter reconnect
- adapter disconnect
- auth/session error
- queue/backpressure warning
- cron trigger delivery
- agent response dispatch

Gateway events should include:

- `trace_id`
- `session_id`
- `channel`
- `chat_id`
- `message_id` when available
- adapter/platform name
- delivery status
- retry count when available

Gateway logs should avoid storing raw message text by default. Store summaries
or truncated previews unless verbose debugging is explicitly enabled.

## Component Placement

### `agent-diva-core::trace`

Own shared trace/log domain types:

```text
agent-diva-core/src/trace/
  mod.rs
  id.rs
  event.rs
  logger.rs
  redaction.rs
  retention.rs
```

Responsibilities:

- `TraceId`
- `TraceEvent`
- event serialization
- redaction helpers
- retention policy types
- append-only JSONL writer boundary

### `agent-diva-agent`

Emit runtime events:

- message received
- LLM call started/completed/failed
- tool call started/completed/failed
- Plan/Todo events after Phase A exists
- cancellation/stop events

### `agent-diva-manager`

Expose debug APIs:

- get recent logs
- export debug bundle
- clear logs
- update debug settings

### `agent-diva-gui`

Expose settings and debug bundle actions. Avoid building a large log viewer in
the first slice.

### Channel/Gateway Crates

Emit gateway inbound/outbound/error events with shared `TraceId` propagation.

## Storage Location

Recommended workspace-local default:

```text
.agent-diva/logs/runtime-YYYY-MM-DD.jsonl
.agent-diva/debug-bundles/
```

If the runtime already uses a user-level data directory for gateway state, the
implementation may mirror logs there as a later option. The MVP should prefer a
single obvious workspace-local location unless gateway deployment requires a
user-level path.

## Implementation Phases

### Phase B0: Trace Types And Writer

Deliverables:

- `TraceId`
- `TraceEvent`
- JSONL writer
- redaction helpers
- retention config model
- unit tests for serialization and redaction

Acceptance:

- A structured event can be written and read back.
- Secrets are redacted before write.
- Oversized fields are truncated.

### Phase B1: Agent Runtime Events

Deliverables:

- Agent loop emits high-signal events.
- Tool execution emits start/completed/failed.
- Provider calls emit started/completed/failed summaries.

Acceptance:

- A single user message produces a correlated trace through LLM and tool calls.
- Logs include duration and status where available.

### Phase B2: Gateway Events

Deliverables:

- Gateway/channel inbound/outbound/error events.
- Trace ID propagation from inbound message into agent runtime.
- Delivery failure logging.

Acceptance:

- A gateway-handled message can be traced from inbound to outbound.
- Delivery errors include enough metadata for debugging without secrets.

### Phase B3: Debug Bundle And Settings

Deliverables:

- Manager API or CLI command to export debug bundle.
- Manager API or CLI command to clear logs.
- Minimal GUI debug settings page if GUI work is in scope.

Acceptance:

- User can export recent redacted logs.
- User can clear logs.
- Retention prevents unbounded log growth.

## First PR Slice

The smallest useful PR should be:

1. Add `agent-diva-core::trace` types.
2. Add append-only JSONL writer.
3. Add redaction and truncation tests.
4. Emit `message_received`, `tool_call_started`, `tool_call_completed`, and
   `tool_call_failed` from the agent runtime.

This gives immediate debugging value without changing GUI, gateway, or provider
behavior yet.

## Risks And Controls

| Risk | Control |
| --- | --- |
| Logging leaks secrets | Redact before write; default provider payload logging off |
| Logging becomes too heavy | One JSONL file, minimal event set, truncation |
| GUI scope grows too large | Settings/export only in first GUI slice |
| Trace IDs do not propagate | Make trace ID part of inbound/runtime event context |
| Logs grow without bound | Retention policy and clear action |
| Debug bundle exposes private data | Bundle uses already-redacted logs and redacted config |

## Acceptance Criteria

- Logs are structured JSONL and stored durably.
- A single `trace_id` can connect gateway/user input, agent runtime, tool calls,
  and final output when those components participate.
- Secrets are redacted by default.
- Large outputs are truncated.
- Logs have a retention/cleanup path.
- A debug bundle can be exported without manually collecting many files.
