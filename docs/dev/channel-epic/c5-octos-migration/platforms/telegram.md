# Telegram adapter specification

Endpoint-level evidence and current-state gaps are in [`telegram-scan.md`](telegram-scan.md).

## Sources and invariants

- Diva source: `agent-diva-channels/src/telegram.rs`; preserve Bot API polling, commands,
  allowlist, proxy behavior, Markdown-to-HTML fallback, and current typing cleanup.
- Octos source: `octos-bus/src/telegram_channel.rs`; adapt media extraction/download,
  `send_with_id`, typing/listening, edit/delete, and authenticated health behavior.
- Keep teloxide. Do not import Octos manager/bus or create a listener task that returns early.

## Target behavior

- Admit private, group, and forum-topic text plus photo/document/audio/video/voice attachments.
- Use Telegram update/message ID for dedup and correlation. Populate `thread_id` from
  `message_thread_id`; reply identity stays explicit.
- Store every downloaded media item through `ChannelAttachmentStore`; enforce Bot API size and
  local C5 limits before Fabric admission.
- Send Text/Markdown with runtime chunking. Reply only on the first logical chunk. Send typed media
  with caption where valid; never skip an unsupported part.
- Implement typing and recording/listening actions, return message IDs, edit text, delete message,
  and finalize an editable stream through the bound message ID.
- `ProbeHealth` uses `getMe`; static token has no token-refresh capability. Polling is not advertised
  as heartbeat/resume.

## Failure and lifecycle rules

- HTML parse failure may retry the same content as plain text once; receipt refers to the actual
  successful message.
- Bot API `retry_after` maps to `RateLimited` and is not slept inside the adapter when C2 pacing can
  schedule it.
- Cancellation stops polling and all typing/listening refresh tasks. `stop` is idempotent.
- Empty allowlist permits all; unauthorized media is not downloaded.

## Required fixtures

1. Private and group/topic updates with stable IDs and reply/thread mapping.
2. Photo, document, audio, video, and voice download into fake content-addressed storage.
3. `sendMessage` HTML success, HTML failure/plain fallback, message ID receipt, and 429.
4. Reply, media upload, typing/listening, edit, delete, and stream-finalize requests.
5. Fabric busy/cancel path that does not commit dedup.
6. `getMe` healthy/auth-failure probes and long-running listener cancellation.

Done means every Telegram `T` in the matrix has an evidence-manifest entry and all other commands
fail unsupported before a Bot API call.
