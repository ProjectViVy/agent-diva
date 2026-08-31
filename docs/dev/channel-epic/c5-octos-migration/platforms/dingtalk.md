# DingTalk adapter specification

Endpoint-level evidence and current-state gaps are in [`dingtalk-scan.md`](dingtalk-scan.md).

## Sources and invariants

- Diva source: `agent-diva-channels/src/dingtalk.rs`; retain Stream connection registration,
  WebSocket/heartbeat path, group/direct policy, dedup, access token, Markdown, and current media
  upload/send behavior.
- Octos source: `octos-bus/src/dingtalk_channel.rs`; use only parsing, signature, response, and test
  ideas that apply to the Stream transport. Its webhook/sessionWebhook path is not a replacement or
  fallback.

## Target behavior

- Admit group/direct text and rich/Markdown payloads with message ID, conversation identity, sender,
  and typed image/audio/video/file attachments.
- Respect DM/group policy before media fetch. Empty `allow_from` retains allow-all only where the
  selected DingTalk policy permits it.
- Send Text/Markdown and typed media through existing robot/OpenAPI calls. The platform has no C5
  reply/edit/delete/reaction/stream-finalize capability; those commands fail before HTTP.
- Cache and refresh the access token with single-flight behavior. Stream heartbeat and health are
  advertised; generic resume is false unless a future protocol fixture proves a stable resume token.

## Failure and lifecycle rules

- Stream callback ACK is issued only after bounded ingress admission succeeds within the required
  deadline. Admission failure must request redelivery or terminate the connection for retry.
- A media failure is not logged-and-skipped; the command returns a failed/partial diagnosis and does
  not claim a complete receipt.
- Preserve platform response identifiers when available. Otherwise return `Accepted` without a fake
  message ID.
- Cancellation stops WebSocket, heartbeat, token wait, media upload, and reconnect backoff.

## Required fixtures

1. Stream registration, system heartbeat, group/direct callback, ACK, and duplicate callback.
2. Policy combinations and unauthorized media event with zero download/admission.
3. Markdown plus image/audio/video/file upload/send and partial media failure.
4. Token cache/expiry/single-flight/auth retry and 429/Retry-After.
5. Fabric busy before ACK, heartbeat timeout, listener exit, stop, and health transitions.
6. All false interaction/reply/Card commands returning unsupported with zero HTTP calls.

Done means DingTalk retains the Diva Stream feature set, gains typed evidence and receipts, and does
not regress to the simpler Octos webhook channel.
