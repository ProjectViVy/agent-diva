# Feishu/Lark Gate 2 implementation evidence

Implementation commit: the focused worker commit that adds this adapter,
fixtures and evidence note; the handoff message records its exact hash.

The implementation is an independent native `ChannelAdapter` in
`agent-diva-channels/src/adapters/feishu.rs`. It does not call or wrap
`FeishuHandler`, import the Octos runtime, or change `AdapterContext`.

## Frozen provenance

- Octos reference: SHA `5ea987813de4fd2afdd1d78f2106ad2868f0d923`, tag
  `v2.0.3-rc.9`, Apache-2.0.
- Ported endpoint behavior: `octos-bus/src/feishu_channel.rs` symbols
  `base_url_for_region`, webhook signature/decrypt helpers,
  `download_feishu_media`, image/file upload, send/reply, `send_with_id`,
  edit and delete.
- Retained DIVA behavior: `agent-diva-channels/src/feishu.rs` protobuf frame
  codec, ACK frame shape, heartbeat/reconnect loop, card/table rendering,
  message dedup and best-effort `THUMBSUP` seen reaction.
- Transformation: all platform operations use the C5 `ChannelAdapter`,
  `AdapterServices` attachment seam, typed `ChannelEnvelopeV1` identity and
  truthful `DeliveryReceipt`; no Octos bus/manager/default-success method is
  copied.

## Capability and security behavior

The static declaration matches the frozen Feishu matrix: typed text/Markdown,
thread, group/direct, inbound attachments and dedup; text/Markdown/reply/image/
file/card egress; edit/delete/finalize; health, heartbeat, token refresh,
pacing and supervised restart. Audio/video egress, chunking, typing/listening,
generic reaction and resume remain unsupported.

Inbound authorization is evaluated before resource download. A bounded local
admission permit and Fabric admission protect the ACK deadline; a dedup key is
reserved during processing and committed only after Fabric admission succeeds.
The envelope records `ExternalUser`, app, sender, chat, chat type, thread/root,
reply and platform message IDs. Media bytes are bounded, MIME-labelled and
stored through `ChannelAttachmentStore`; remote URLs and absolute paths are
never emitted.

Region is explicit in `FeishuRegion` and maps China to `open.feishu.cn`, Global
and Lark to `open.larksuite.com`. The present shared `FeishuConfig` has no
region field, so the crate-visible production constructor defaults to China;
the region-aware constructor is ready for a later shared-schema/C6 change.

Webhook URL verification is the only unsigned path. Ordinary webhook payloads
require timestamp, nonce, signature and encrypt key. Signature mismatches,
missing headers, invalid base64, AES-256-CBC failures and bad padding are
fail-closed typed errors. The normal C5 listener remains the DIVA protobuf WebSocket;
these helpers do not introduce a webhook fallback listener.

## Fixture/test mapping

| Evidence | Fixture/test | Expected proof |
| --- | --- | --- |
| Protobuf event and identity | `protobuf-event.json`; `duplicate_events_are_not_admitted_twice` | One typed external envelope; replay does not enter Fabric twice |
| Protobuf ACK | `protobuf-frame.json`; `protocol_and_webhook_fixtures_are_parseable` | ACK preserves frame and emits `biz_rt=0` with HTTP 200 payload |
| Region | `region-endpoints.json`; `region_defaults_are_explicit_and_distinct` | China/global/Lark mapping is deterministic |
| Webhook verification | `webhook-url-verification.json`; `webhook_signature_is_fail_closed` | URL challenge can validate token; missing/bad signature rejects |
| AES encrypted webhook | `webhook-encrypted-event.json`; `aes256_decrypt_round_trip_and_bad_padding_fail_closed` | AES-256-CBC/PKCS#7 decrypts valid payload and rejects tampering |
| Typed media | `media-resource.json`; `media_fixture_is_stored_as_typed_attachment_after_authentication` | GET resource bytes are stored as `AttachmentRef` with bounded content |
| Send/reply IDs | `send-responses.json`; `send_fixture_returns_real_message_id_and_reply_uses_bound_endpoint` | Send and reply use exact target and return platform message IDs |
| Deadline/lifecycle | `ack-deadline.json`; listener constants and cancellation branches | Admission/reconnect/heartbeat bounds are explicit and cancellable |

## Known handoff limits

- Shared `adapters/mod.rs`, `lib.rs` and the native factory are Lead-owned and
  intentionally absent from this worker commit; Lead wires the module and
  constructor during fixed Gate 3 integration.
- Production region selection requires a future shared config decision; no
  mutable endpoint environment override is added here.
- Reaction remains the retained best-effort seen marker only. It is not
  advertised as `InteractionReaction` and cannot make inbound admission fail.
