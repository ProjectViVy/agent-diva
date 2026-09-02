# Scan playbook

The scan is evidence collection, not a reason to redesign the whole workspace. Use `rg` and direct
file reads; do not run code generators or formatters during this phase.

## 1. Agent Diva contract scan

Inspect and record the relevant symbols from:

- `agent-diva-core/src/channel/mod.rs`: envelope, address, correlation, typed parts, receipt.
- `agent-diva-core/src/channel/runtime_contract.rs`: capability set, limits/probe, commands, health.
- `agent-diva-core/src/channel/fabric.rs`: bounded admission and cancellation semantics.
- `agent-diva-channels/src/adapter.rs`: native trait and error contract.
- `agent-diva-channels/src/runtime/{registry,pacing,supervisor}.rs`: behavior already solved by C2.
- The six active legacy platform files: transport, auth, allowlist, dedup, media, stop, and tests.
- `agent-diva-core/src/config/schema.rs`: credentials and policy fields that are actually consumed.
- `agent-diva-manager/src/file_service.rs`: content-addressed attachment authority.
- `agent-diva-manager/src/runtime/bootstrap.rs`: future C6 assembly point.

For each platform, produce a table containing current inbound event types, outbound endpoints,
platform IDs, rate-limit behavior, reconnect state, token state, allowlist semantics, hardcoded
endpoints, blocking calls, detached tasks, and existing tests.

## 2. Octos scan

Pin the snapshot before reading. Primary files under `crates/octos-bus/src/` are:

- `channel.rs`
- `telegram_channel.rs`
- `discord_channel.rs`
- `feishu_channel.rs`
- `dingtalk_channel.rs`
- `email_channel.rs`
- `qq_bot_channel.rs`

Also inspect `crates/octos-cli/src/commands/gateway/adapters/` and
`crates/octos-cli/src/stream_reporter.rs` for construction and stream/thread behavior.

Record concrete symbols and tests, not summaries. For every candidate, classify it as:

- `port`: protocol behavior can be reimplemented with the current Diva transport and contracts;
- `adapt`: useful behavior needs a Diva-native representation or stronger failure contract;
- `retain-diva`: the existing Diva path is stronger and must not regress;
- `reject`: no-op default, duplicate runtime, incompatible dependency, unsafe assumption, or out of
  scope.

## 3. Required comparison questions

For all six platforms answer:

1. Does listener `start` remain long-running and cancellation-aware?
2. Which platform identity becomes `message_id`, `reply_to`, and `thread_id`?
3. Does dedup commit before or after bounded Fabric admission?
4. What response proves a real receipt, and is the honest state `Accepted` or `Delivered`?
5. Which media types can be downloaded/uploaded with size, MIME, and digest validation?
6. Which operations are truly idempotent, and which need a supplied idempotency key?
7. How are 429/Retry-After, token expiry, invalid sessions, and heartbeat failure mapped?
8. Does empty `allow_from` preserve the current allow-all contract? What platform-specific policy
   further restricts it?
9. Can any external payload forge owner context or origin? The required answer is no.
10. What exact fixture or local mock proves every target-true capability?

## 4. Output gate

The scan is complete only when the provenance ledger and all six platform specifications contain
file/symbol/test references, the capability matrix has no `TBD`, and every mandatory enhancement
has an implementable wire path. Missing optional evidence means `false`; missing mandatory evidence
is a blocker.

## 5. C5-P2 deep-scan output contract

The six independent reports under `platforms/*-scan.md` are the endpoint-level source for the
implementation handoff. A worker must not infer a capability from a trait method or a README. For
each operation, copy this record shape into the channel report and then link it from
`endpoint-ledger.md`:

| Field | Required content |
| --- | --- |
| Octos symbol | Fixed-SHA path, module, function/type, line or stable code anchor |
| Diva symbol | Current path, function/type, line or stable code anchor |
| Wire operation | HTTP method/path, WS opcode/frame, or IMAP/SMTP command |
| Auth | Header/query/body credential shape and redaction rule |
| Request | JSON/form/multipart fields, content type, limits |
| Success | Status/body fields, platform ID, receipt state |
| Failure | Status/body, retryability, Retry-After, timeout and backoff |
| Identity | sender/chat/thread/message/reply IDs and origin |
| Policy | allowlist, DM/group/mention checks and order relative to media |
| Decision | `Port`, `Adapt`, `Retain-DIVA`, `Reject`, or `Blocked` with owner |
| Proof | fixture/mock/transcript, Rust test name, expected request/response |

The report must scan source code, configuration/registration, docs, and tests on both sides. It
must explicitly classify “implemented but unproven” as `Partial`, not `Implemented`. The current
root C5 target files (`agent-diva-core/src/channel/**`, `agent-diva-channels/src/adapter.rs`) are
planned contracts in this branch; if absent, record that fact and do not describe them as existing.

## 6. Mandatory cross-cutting pass

After the per-channel scan, the coordinator must update `cross-cutting-gap-matrix.md` for:

- typed image/file/audio attachments and provider vision capability;
- group chat identity, mention, thread and policy behavior;
- approval routing through Manager/Sandbox/Ask User with owner-proof and expiry;
- `allow_from` and platform-specific permission order;
- dedup after bounded admission, backpressure, pacing and cancellation;
- real message-ID receipts, edit/delete/reaction and unsupported zero-side-effect behavior.

Any item without a wire path and fixture remains `Missing` or `Blocked` in the evidence manifest.
