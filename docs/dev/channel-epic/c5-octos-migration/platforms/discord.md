# Discord adapter specification

## Sources and invariants

- Diva source: `agent-diva-channels/src/discord.rs`; preserve the current direct HTTP/Gateway
  implementation, allowlist, guild filter, mention-only policy, bot filtering, attachment limits,
  heartbeat sequence, configured gateway URL, and reply behavior.
- Octos source: `octos-bus/src/discord_channel.rs`; adapt send-ID parsing, edit/delete, add/remove
  reactions, and embed behavior. Do not add Serenity merely to mirror Octos.

## Target behavior

- Admit DM and guild messages, Markdown content, stable Discord snowflakes, attachment parts, and
  thread/channel identity. A thread channel ID is the chat; the root/referenced message is explicit
  correlation, never a mutable current-thread map.
- Preserve Gateway heartbeat and implement Resume with the last acknowledged sequence/session ID;
  invalid session clears resume state and produces a diagnosable reconnect.
- Send Text/Markdown with 2,000-character runtime chunking, first-chunk reply reference, and real
  message IDs. Implement multipart media, embeds for supported Card payloads, typing, edit/delete,
  reaction add/remove, and editable stream finalize.
- Health combines local Gateway heartbeat/ACK state with the existing lightweight authenticated
  Gateway REST probe; static bot tokens do not advertise refresh.

## Failure and lifecycle rules

- Parse JSON/body `retry_after` and HTTP header forms; return `RateLimited` instead of sleeping in
  platform code.
- Preserve mention and sender policy before attachment fetch.
- Heartbeat timeout, reconnect opcode, invalid session, close, cancellation, and `stop` each have a
  deterministic state transition. No detached heartbeat survives the listener.
- A successful REST create returns `Accepted` plus the response snowflake.

## Required fixtures

1. Gateway Hello/Identify/Ready, DM and guild `MESSAGE_CREATE`, mention filtering, attachments.
2. Heartbeat/ACK, resumable reconnect, server reconnect, and invalid-session identify fallback.
3. Text chunks with first-chunk reply and captured message IDs.
4. Multipart image/audio/video/file, embed Card, typing, edit, delete, reactions, finalize.
5. 429 in header/body formats, 4xx permission error, transport close, cancellation, and dedup.
6. Capability-false commands and unsupported attachment types with zero REST calls.

Done means the worker proves both Gateway reliability and all declared REST capabilities without a
new SDK or process-global endpoint override.
