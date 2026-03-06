# Summary

## Scope

Refined GUI stop flow to be session-aware instead of hardcoded `api:default`.

## Changes

- Added frontend runtime session identity:
  - `currentChannel` (default `gui`)
  - `currentChatId` (generated per chat context)
- `send_message` now sends `channel` and `chat_id` with each request.
- `stop_generation` now targets the same active `channel/chat_id`.
- When user clears chat UI, a new `chat_id` is generated to avoid cross-session stop targeting.

## Outcome

Stop now targets the current GUI conversation instead of a fixed global key, improving correctness for multi-session usage patterns.
