# Summary

## Scope

Implemented `/stop` text-command interruption across core entry paths while preserving stop-only semantics.

## Delivered

- API `/api/chat` now recognizes `message == "/stop"` and routes to manager stop flow.
- CLI/TUI now supports `/stop` command:
  - Local mode sends runtime control stop command for current session.
  - Remote mode calls `/api/chat/stop` with current session target.
- GUI text input now maps `/stop` to existing `stopMessage()` behavior.
- Telegram channel supports `/stop` command and `/stop@bot` text shortcut, forwarding to manager stop API.
- Architecture report updated with `/stop` semantics and boundaries versus `/new`/`/reset`.
