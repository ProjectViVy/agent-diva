# Capability TCK, fixtures, and QQ live smoke

## Evidence manifest

Create `agent-diva-channels/tests/fixtures/c5/capability-evidence.json`. Each record contains:

- adapter and exact `ChannelCapability`;
- target value from the frozen matrix;
- Rust test name;
- fixture/mock path;
- proof class: ingress, egress, interaction, or reliability;
- expected request/response and receipt/error outcome.

A global TCK instantiates every adapter and asserts that its effective static snapshot equals the
matrix. Every `true` record must reference an existing passing test. Every applicable `false`
command is executed against a transport spy and must return `UnsupportedCapability` with zero calls.

## Fixture layout

Use `tests/fixtures/c5/{telegram,discord,feishu,dingtalk,email,qq}/`. Keep payloads minimal,
deterministic, redacted, and license-safe. Prefer JSON/Protobuf bytes or RFC 822 transcripts captured
from public protocol shapes, not production traffic.

- HTTP: local mock server, exact method/path/query/body/header assertions, redacted auth matching.
- WebSocket: loopback listener with scripted frames, heartbeat timing under paused Tokio time where
  practical, clean server shutdown.
- Email: fake IMAP/SMTP traits/transcripts; no network and no global environment variables.
- Attachments: in-memory `ChannelAttachmentStore`, boundary sizes, MIME validation, SHA-256, readback.
- Fabric: real bounded `FabricKernel` handle/consumer to prove admission, cancellation, ordering, and
  no early dedup commit.

## Mandatory scenario families

1. Valid inbound text/media mapping and unauthorized/malformed rejection.
2. Message/thread/reply identity under concurrent chats; no sticky cross-turn inference.
3. Text/Markdown/Card downgrade, chunk order, first-chunk reply, media, real message-ID receipt.
4. Every supported interaction plus unsupported zero-side-effect cases.
5. 429/Retry-After, safe/unsafe retry, auth refresh, partial delivery, permission and malformed body.
6. Listener clean exit/error/panic, heartbeat, resume, stop during read/backoff/token wait, health.
7. Secret/identifier redaction and attachment size/path/MIME/digest safety.

## QQ live harness

Provide an ignored, explicit live harness that uses official endpoints and the native `QQAdapter`
mounted in the real Fabric, pacing lane, and supervisor. It must never run during ordinary tests.

Credentials and target IDs are supplied outside the repository through these environment variables:

- `AGENT_DIVA_LIVE_QQ_APP_ID`
- `AGENT_DIVA_LIVE_QQ_SECRET`
- `AGENT_DIVA_LIVE_QQ_C2C_OPENID`
- `AGENT_DIVA_LIVE_QQ_GROUP_OPENID`

The harness validates:

1. C2C inbound reaches Fabric exactly once.
2. Group @ inbound reaches Fabric exactly once.
3. Replayed duplicate event is not admitted twice.
4. A reply uses the frozen C2C/group address and platform message ID.
5. Streaming degrades to one final send because QQ edit/finalize capability is false.
6. The receipt is `Accepted` with a real platform response message ID.
7. A controlled disconnect resumes/reconnects without crossing chat identity.
8. Stop terminates listener, heartbeat, token work, and backoff.

Logs and verification records show only hashed/truncated IDs and status observations. They never show
tokens, secret, full open IDs, media URLs, or raw message content. If credentials or permissions are
unavailable, document the blocked command and leave C5-V/TODOLIST open.

## Required gates

- `cargo test -p agent-diva-channels`
- focused platform and global capability TCK tests
- `cargo clippy -p agent-diva-channels --all-targets -- -D warnings`
- `just msrv-probe check -p agent-diva-channels`
- `just fmt-check`
- `just check`
- `just test`
- `git diff --check`

Record commands, counts, failures/reruns, live environment class, and observation points in the C5
verification log. A passing method-existence test is never sufficient evidence.
