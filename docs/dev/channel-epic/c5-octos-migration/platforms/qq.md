# QQ adapter specification

## Sources and invariants

- Diva source: `agent-diva-channels/src/qq.rs` and
  `agent-diva-channels/tests/qq_reconnect_integration.rs`; retain official OpenAPI/Gateway,
  native-TLS Windows behavior, token handling, heartbeat, invalid-session backoff, and current C2C
  allowlist semantics. Eliminate process-global test overrides in the new adapter.
- Octos source: `octos-bus/src/qq_bot_channel.rs`; adapt C2C/group event parsing, group/C2C send
  selection, session sequence/resume, and dedup structure.
- Octos media is not sufficient for C5; implement typed media against the official QQ operations
  already represented by the pinned research/fixtures, without inventing unsupported Guild scope.

## Target behavior

- Admit C2C and group @ messages with the correct open IDs, group/chat identity, stable message ID,
  content, and typed image/audio/video/file attachments. Guild/channel events remain false unless a
  separately reviewed matrix change provides complete evidence.
- Dedup by platform message ID only after Fabric admission. Resume uses the last acknowledged
  sequence and session ID; invalid session clears resume state with bounded cooldown/backoff.
- Select C2C or group send endpoint from the frozen address, include explicit reply message ID,
  upload/send typed media, and parse the real response message ID into an `Accepted` receipt.
- Apply runtime chunking to text. QQ does not advertise Markdown, thread, typing, edit/delete,
  reaction, Card, or editable stream; the planner sends one final response after stream downgrade.
- Token refresh is single-flight and retries an auth-expired safe command once. Heartbeat ACK and
  health state are explicit.

## Failure and lifecycle rules

- Never infer C2C/group target from the most recent inbound message. The `ChannelAddress` and
  correlation frozen on the command are authoritative.
- Group permission errors, media upload failure, 429, auth expiry, invalid session, missed heartbeat,
  duplicate event, Fabric busy, and cancellation each have stable typed outcomes.
- Do not log app secret, access token, full open IDs, media URLs, or raw payloads. Live reports use
  hashes/truncated identifiers.
- Cancellation terminates read, heartbeat, token wait, media upload, reconnect delay, and cooldown.

## Required fixtures

1. Gateway Hello/Identify/Ready, C2C message, group @ message, Ping/Pong, heartbeat/ACK.
2. Resume after disconnect, reconnect request, resumable/non-resumable invalid session, cooldown.
3. C2C/group dedup and Fabric busy proving the processed marker is not committed early.
4. C2C/group text reply, chunking, real response IDs, image/audio/video/file upload/send.
5. Token cache/expiry/single-flight/auth retry, 429, permission, partial media, and health.
6. Final-only stream downgrade plus all false commands returning unsupported before OpenAPI calls.

## Live gate

QQ is the user-selected vertical sample. The runbook in `../06-tck-fixture-and-live-smoke.md` is
mandatory. Without external credentials and permissions, leave C5-V open and do not claim C5 done.
