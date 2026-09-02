# Feishu adapter specification

Endpoint-level evidence and current-state gaps are in [`feishu-scan.md`](feishu-scan.md).

## Sources and invariants

- Diva source: `agent-diva-channels/src/feishu.rs`; retain tenant token cache, Protobuf WebSocket,
  immediate protocol ACK requirements, heartbeat, event/message dedup, image download, interactive
  Markdown cards, allowlist, and table rendering.
- Octos source: `octos-bus/src/feishu_channel.rs`; adapt mock-server structure, message-ID parsing,
  reply endpoint, image/file upload, edit, and delete.
- Do not add webhook fallback or user endpoint configuration in C5.

## Target behavior

- Admit group/direct text, rich post Markdown, stable event/message IDs, thread/root/reply identity,
  and image/file typed attachments.
- Obtain bounded Fabric admission before final event ACK where the platform permits; admission
  deadline must remain below Feishu's ACK deadline. Busy/closed admission causes retry/reconnect,
  not ACK-and-drop.
- Send Markdown as an interactive card, plain text when requested, reply through the reply endpoint,
  upload image/file, and parse the returned message ID. Implement edit/delete and finalize through
  the original bound message.
- Refresh tenant tokens under a single-flight lock before expiry; one auth-expired response may
  invalidate and retry only when the command is retry-safe.
- Heartbeat and health state are explicit. Feishu does not advertise generic resume, reaction, audio,
  or video in C5.

## Failure and lifecycle rules

- Reaction currently used as a best-effort seen marker is not a declared command capability and may
  not turn an inbound success into failure.
- Protocol ACK, token, media, card, reply, edit/delete, 429, permission, and malformed response
  failures use stable codes with secrets and signed URLs redacted.
- Cancellation terminates WebSocket read, heartbeat, token waits, and pending reconnect.

## Required fixtures

1. Protobuf connect/start frame, heartbeat, ACK, duplicate event, direct/group message.
2. Text/rich post plus image/file download into fake attachment storage.
3. Token success/cache/expiry/single-flight/auth retry.
4. Send/card response ID, reply endpoint, image/file upload, edit, delete, finalize.
5. ACK deadline under normal admission and deliberate Fabric busy without dedup commit.
6. 429, permission, malformed body, heartbeat timeout, stop, and health transitions.

Done means the stronger Diva WS path remains intact while Octos reply/media/edit/delete evidence is
represented by typed commands and receipts.
