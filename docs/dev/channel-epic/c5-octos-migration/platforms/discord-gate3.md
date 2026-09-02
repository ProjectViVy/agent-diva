# Discord C5-V Gate 3 evidence

状态：`partial`。本轮完成 Discord native adapter 的最小可审计修复闭环：源码、真实
平台形状的脱敏 fixture、可执行 HTTP/WebSocket wire test 和本页证据均已更新。四个
分配行仍不升级为平台能力的 `verified`，因为没有 live Discord credential/connection
或 C2 supervisor 的外部 retry/pacing transcript；没有改动冻结 capability matrix。

## 审计基线与边界

- Octos fixed checkout：`C:\Users\Administrator\Desktop\morediva\.workspace\octos`，
  `HEAD=5ea987813de4fd2afdd1d78f2106ad2868f0d923`，仅作为源码行为对照。
- Octos anchors：`crates/octos-bus/src/discord_channel.rs` 的
  `Handler::message`、`DiscordChannel::start`、`send_with_id`，以及
  `crates/octos-bus/src/dedup.rs::MessageDedup`。
- Octos Serenity 封装了 Gateway lifecycle；它没有在该 checkout 中提供可单独审计的
  官方 op6/op7/op9 wire transcript。因此本页的 Gateway wire 证据只归因于 DIVA
  native `DiscordAdapter` 的测试 fixture，不把 Serenity library behavior 臆造为
  Discord 官方 wire evidence。
- DIVA frozen snapshot 仍只声明现有 Discord capabilities；`ReliabilityResume` 没有
  被加入，且本轮没有改 `ChannelCapability`、manifest 或共享 TCK。
- 官方形状参照：[Gateway opcodes/status codes](https://docs.discord.com/developers/topics/opcodes-and-status-codes)、
  [Gateway lifecycle](https://docs.discord.com/developers/events/gateway)、
  [Create Message multipart and message reference](https://docs.discord.com/developers/resources/message)。

## Disposition summary

| Row | Frozen target | DIVA evidence | Disposition | Remaining gap |
| --- | --- | --- | --- | --- |
| DC-01 | heartbeat + supervised restart; no `ReliabilityResume` atom | `consume_gateway`, `send_gateway_heartbeat`, `gateway_close_error`; Gateway lifecycle fixture/tests | `partial` | live Gateway and external supervisor retry/backoff transcript |
| DC-02 | text/markdown/thread/group/direct/dedup ingress | `parse_incoming_message`, ordered `handle_incoming`, `MessageDedup`; policy and Fabric tests | `partial` | live DM/guild policy observation and production event stream |
| DC-03 | typed ingress and image/audio/video/file egress | `fetch_attachment`, `send_attachments`, content-addressed validation; four-kind HTTP wire tests | `partial` | live CDN/media and oversize/cancel matrix; no path-based Octos behavior claimed |
| DC-05 | health | `parse_response`, `parse_rate_limit`, `rest_error`, `probe_health`; 429/403/malformed/timeout tests | `partial` | external retry executor and live permission/health response |

All four rows remain conservative `partial`; the deterministic adapter proof is not a claim
that Discord production access was exercised.

## DC-01 — Gateway HELLO/IDENTIFY/READY, heartbeat, resume and lifecycle

### Source and endpoint evidence

- `discover_gateway` calls `GET /gateway/bot` with `Authorization: Bot <token>` and bounds
  the discovery body. `next_gateway_url` uses READY's `resume_gateway_url` only while a
  session is retained; otherwise it discovers a fresh URL.
- `consume_gateway` requires the first frame to be op `10` HELLO with a positive integer
  `heartbeat_interval`, then sends official-shaped op `2` IDENTIFY (`token`, `intents`,
  `properties.os/browser/device`). READY stores `session_id`, `resume_gateway_url`, the
  bot user ID, and sequence.
- `send_gateway_heartbeat` sends op `1` with the latest sequence and tracks the pending
  ACK. A second scheduled heartbeat without op `11` returns retryable
  `Execution{code="heartbeat_ack_timeout"}` and sends a close frame.
- op `6` RESUME carries `token`, `session_id`, and `seq`; op `7` RECONNECT closes the
  socket and returns retryable `gateway_reconnect`; op `9` INVALID_SESSION preserves or
  clears session state from its boolean `d` and returns retryable
  `gateway_invalid_session` with the configured cooldown. `gateway_close_error` applies
  close-code retry policy and clears state for a new-session close.
- `start` and `stop` share cancellation, send close code `1000` with a bounded close send,
  and set lifecycle health to `Down` after stop. `run_gateway` bounds connect, uses
  cancellation-aware delay, and exposes retryable transport/lifecycle errors to the
  supervisor. `ReliabilityResume` remains absent from the capability set by design.

### Fixture and passing wire tests

- `agent-diva-channels/tests/fixtures/c5/discord/gateway-frames.json` now contains
  redacted HELLO, IDENTIFY, READY, MESSAGE_CREATE, heartbeat/ACK, RESUME/RESUMED,
  RECONNECT and INVALID_SESSION shapes. The fixture-shape test checks the opcode and
  required field positions; it is also still parseable by the shared fixture sanity test.
- `wire_discord_gateway_handles_hello_identify_ready_heartbeat_and_cancel` runs a local
  WebSocket server. It asserts the outbound IDENTIFY token/intents/properties, heartbeat
  sequence, one ingress envelope, and a stop close transcript.
- `wire_discord_gateway_resumes_after_reconnect_and_stops_cleanly` accepts two Gateway
  connections, sends READY then op7, requires op6 RESUME with the saved session/sequence,
  sends RESUMED, and asserts the final cancellation close. Discovery is requested once,
  proving the retained resume URL is used on reconnect.
- `gateway_heartbeat_timeout_and_invalid_session_have_typed_lifecycle_errors` exercises
  no-ACK close/typed timeout and op9 false/cooldown/close behavior directly against local
  WebSocket fixtures.

## DC-02 — DM/guild/thread/mention/bot filter and dedup

- `parse_incoming_message` accepts Discord message objects, preserves `channel_id`,
  `guild_id`, thread channel/type `10..12`, `message_reference.message_id`, and collects
  mention IDs rather than treating any mentioned bot as this bot.
- `handle_incoming` checks the READY bot ID/self-message, bot policy, allowlist, guild
  policy, and guild mention policy before reserving dedup or fetching media. A configured
  guild still permits `guild_id=None` DMs. Empty allowlist remains allow-all through the
  shared helper.
- `MessageDedup` is an atomic mutex-protected reserve/commit/release cache with the Octos
  1000-entry/60-second shape. IDs are committed only after Fabric admission; failures
  release the reservation so a later delivery may retry.
- `ingress_policy_handles_dm_guild_mentions_bots_and_atomic_dedup` proves DM admission,
  wrong-guild rejection, wrong-mention rejection, configured sender exception, self/bot
  filtering, and concurrent duplicate admission of exactly one envelope. The assertion
  observes the Fabric response, not just parser output.
- `wire_discord_gateway_handles_hello_identify_ready_heartbeat_and_cancel` additionally
  proves the official-shaped MESSAGE_CREATE event preserves thread and standard reply
  correlation through the Gateway path.

The remaining gap is production guild/DM permission and event-stream observation; no
synthetic mention is treated as live Discord behavior.

## DC-03 — typed attachments and multipart egress

- `fetch_attachment` checks Discord's declared size before transport, checks the response
  status, bounds `Content-Length` and streamed bytes at 25 MiB, honors cancellation, and
  rejects a declared/returned size mismatch with a typed execution code.
- `handle_incoming_reserved` chooses declared/response/filename MIME, stores bytes through
  the existing content-addressed AttachmentStore, validates the returned reference, and
  emits `ContentPart::Image`, `Audio`, `Video`, or `File`. The public content contract is
  unchanged.
- `send_attachments` reads and validates stored bytes, uses one Discord multipart request
  with `payload_json`, `attachments` descriptors, `files[0..n]`, filename and MIME. Text,
  card and standard `message_reference` are carried in that same payload; the receipt
  returns the real response message ID.
- `wire_discord_ingress_downloads_typed_attachments_before_admission` uses four local CDN
  GET responses and asserts four typed Fabric parts plus each request path.
- `wire_discord_rest_preserves_reply_embed_and_attachment_receipts` asserts one outbound
  multipart request for image/audio/video/file, MIME and filenames for every `files[n]`,
  standard `message_reference`, and the accepted snowflake receipt. It also retains the
  edit/delete/reaction/typing response assertions.
- `rest-responses.json` records redacted message/health/rate-limit/permission and
  multipart shapes; the fixture-shape test parses and checks the multipart reference.

This is an AttachmentRef/ContentPart implementation, not a claim that DIVA has copied
Octos's local path media storage. The remaining gap is a live CDN and production-size,
cancelled-download matrix.

## DC-05 — rate limits, permissions, malformed responses and health

- `parse_rate_limit` reads both Discord's `Retry-After` header and JSON `retry_after` body;
  a valid body takes precedence, with a one-second fallback. `AdapterError::RateLimited`
  carries the parsed duration for the outer retry policy.
- `rest_error` maps HTTP 403 to typed `Execution` code `permission_denied`, preserves a
  bounded body diagnosis, and marks server errors retryable. Malformed successful message
  JSON returns non-retryable `discord_response`; status-only operations validate HTTP
  success without inventing a message ID.
- `probe_health` calls the Discord gateway discovery endpoint, parses a non-empty `{url}`,
  reports a receipt and `Healthy`, and records `Degraded` for transport, HTTP, malformed,
  or empty-body failures. `send_request` maps reqwest timeout to a retryable typed code and
  all request/read paths honor adapter cancellation.
- `wire_discord_rate_limit_and_unsupported_listening_are_explicit` asserts body-over-header
  429 precedence and proves unsupported Listening returns before a second HTTP request.
- `wire_discord_permission_and_malformed_responses_are_typed` asserts 403
  `permission_denied` and malformed success `discord_response`.
- `wire_discord_health_marks_success_and_malformed_probe` asserts the success receipt and
  health transition `Healthy -> Degraded`; `wire_discord_timeout_is_typed_and_retryable`
  asserts the bounded timeout code and retryable classification.

The adapter returns retryable errors; C2/runtime retry execution and rate-limit pacing are
outside this channel-only change and need Lead-level or integration evidence. No local
retry loop is claimed.

## Validation and handoff

Focused command run in the isolated Discord worktree:

```text
cargo check -p agent-diva-channels
cargo test -p agent-diva-channels adapters::discord::tests --no-fail-fast
```

The focused adapter suite passed `17/17` after the final source/test changes. Full workspace
validation and live Discord testing were not run because the parallel worktree scope forbids
touching other channels/shared governance files and no live credential is available. Lead
should independently run the shared C5 gate and, if credentials are provisioned, capture a
redacted real Gateway/REST transcript before upgrading any row or capability.
