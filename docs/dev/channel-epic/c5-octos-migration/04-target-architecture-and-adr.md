# Target architecture and frozen decisions

## ADR-1: Native implementations live beside, not behind, legacy handlers

Create `agent-diva-channels/src/adapters/` with `telegram.rs`, `discord.rs`, `feishu.rs`,
`dingtalk.rs`, `email.rs`, and `qq.rs`. Public concrete names end in `Adapter`. The existing
top-level `*Handler` files remain available only until C6; the new adapters must not call them or
convert legacy `InboundMessage`/`OutboundMessage`.

Shared pure protocol helpers may be independently extracted only when neither side calls through a
legacy trait. Do not build old-to-new/new-to-old wrappers.

## ADR-2: Keep the C2 adapter and runtime contracts authoritative

`ChannelAdapter` retains `name`, `capabilities`, long-running `start`, `execute`, local `health`,
and `stop`. `start` must await the listener until cancellation or transport failure; detached
spawn-and-return is invalid because the supervisor cannot observe it.

Registry routing, capability rejection, egress capacity, Markdown/Card downgrade, text chunking,
retry safety, Retry-After, partial receipts, panic capture, jittered backoff, and shutdown stay in
the existing C2 runtime. Platform adapters implement protocol behavior only.

## ADR-3: Inject attachment authority at construction

Add the following channel-owned seam without changing `AdapterContext`:

```text
AdapterServices
└─ attachments: Arc<dyn ChannelAttachmentStore>

ChannelAttachmentStore
├─ put(IngressAttachment) -> AttachmentRef
└─ get(AttachmentRef) -> StoredAttachment
```

`IngressAttachment` contains source channel, platform message ID, sender, file name, declared MIME,
and bounded bytes. `StoredAttachment` returns validated bytes plus file name/MIME. The Manager C6
implementation delegates to its single shared `FileService`/`FileManager`; C5 tests use an
in-memory fake.

The resulting `AttachmentRef.uri` is the content ID `sha256:<hex>`; `sha256` carries raw lowercase
hex. No absolute storage path, `.nanobot/media`, unbounded base64, or unauthenticated remote URL is
published into Fabric.

## ADR-4: Construction and future assembly

Expose `build_active_adapters(&Config, AdapterServices)` returning the enabled, ready adapters or a
typed build error. It does not register or start them. C6 supplies Manager services, registers each
adapter, creates pacing/supervisor handles, and removes `ChannelManager`.

Official endpoints remain code constants. Unit-test-only endpoint structs are private to adapter
modules; do not add generic product base URLs or mutable process-wide environment overrides.

## ADR-5: Identity and ingress ordering

- `origin` is always `ExternalUser`; external payloads cannot carry owner turn context.
- `session_key` is `{channel}:{chat_id}` for the current single-account config model.
- `account_id`, `sender_id`, `chat_id`, and platform thread/topic/root ID populate `ChannelAddress`.
- Platform message ID populates `Correlation.message_id`; reply/root populates `reply_to` and, when
  the platform has a stable thread, `thread_id`.
- Dedup checks may precede parsing, but the processed marker commits only after bounded Fabric
  admission succeeds. A busy/closed Fabric path must produce a retryable diagnosis or reconnect,
  never ACK-and-drop.
- Feishu/DingTalk ACK deadlines are honored by using a bounded admission deadline below the
  platform limit; admission failure must cause the platform to retry rather than lie about success.

## ADR-6: Access control and secrets

Preserve the current active-channel contract: empty `allow_from` allows all; a populated list
restricts senders. Preserve Discord mention/guild filters and DingTalk DM/group policy. Reject
unauthorized input before downloading media or admitting Fabric. Never log tokens, secrets, email
passwords, signed URLs, full attachment bytes, or unredacted live identifiers.

## ADR-7: Commands, downgrade, and receipts

Every adapter exhaustively matches `ChannelCommand`. A false capability returns
`UnsupportedCapability` before any transport call. Platform 429 maps to `RateLimited`; retryable
transport/auth failures use stable codes and optional retry hints.

An ordinary successful send is `Accepted` unless a platform supplies an explicit delivery
confirmation. Parse and return the real platform message ID whenever the platform exposes one.
Consent-disabled email, skipped media, empty/malformed recipients, or failed optional attachments
are not success. Editable-stream adapters create or edit the bound message and finalize it once;
non-editable adapters receive only the final message from the pacing planner.

## ADR-8: Probes and effective capabilities

Each adapter owns a static declaration, optional runtime `ChannelCapabilityProbe`, and local
`ChannelHealth`. `capabilities()` returns static merged with the latest probe. `ProbeHealth`
performs the cheapest authenticated platform check, updates health/probe state, and returns a
receipt/diagnosis; the synchronous `health()` performs no network I/O.

Users cannot force a capability true in config. `Unknown` health is not proof of
`ReliabilityHealth`.

## ADR-9: Compatibility and dependencies

- Keep Rust 1.80 and edition 2021.
- Do not import Octos bus/runtime/config or a platform SDK solely for parity.
- Prefer existing teloxide, reqwest, tokio-tungstenite, lettre/imap, prost, and native-TLS paths.
- New dev dependencies must be workspace-managed where available and justified by a fixture.
- Any non-trivial Octos-derived code records Apache-2.0 provenance in the ledger and commit.

## C5-P2 cross-cutting implementation contract

The deep-scan reports establish four additional invariants before any platform worker writes code:

1. **Multimodal ingress**: authorization and bounded Fabric admission precede remote media fetch;
   fetched bytes enter the content-addressed attachment seam, then context assembly decides whether
   the configured provider can perform image recognition. A URL, absolute path, marker, or empty
   placeholder is not an image capability proof.
2. **Group identity**: `chat_type`, group/chat ID, sender ID, mention state, thread/reply ID and
   platform message ID are explicit in every group event. The adapter never infers an outbound group
   destination from the last inbound message.
3. **Approval**: Manager/Sandbox/Ask User remains the authority. Channel cards, keyboards or text
   commands only transport a signed/expiring decision reference; a platform sender cannot assert
   owner context. Unsupported approval UI is an explicit typed outcome.
4. **Permission order**: empty `allow_from` retains the executable DIVA allow-all behavior; a
   populated list and platform-specific DM/group/mention policy are evaluated before media,
   admission, approval and tools. The stale `agent-diva-channels/agents.md` deny-by-default claim
   must be corrected during documentation review.

These are documentation-frozen target rules. If `agent-diva-core/src/channel/**` or
`agent-diva-channels/src/adapter.rs` is absent in the worker's base commit, the worker must first
complete the shared Gate 1 contract; it must not create a wrapper around the legacy handler.
