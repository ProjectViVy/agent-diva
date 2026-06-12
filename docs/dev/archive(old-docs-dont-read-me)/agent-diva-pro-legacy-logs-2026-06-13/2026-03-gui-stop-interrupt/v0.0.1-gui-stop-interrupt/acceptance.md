# Acceptance

## User-visible Checks

1. Open GUI chat page.
2. Send a message that triggers streaming output.
3. Confirm `Stop` button (left of `Send`) becomes clickable during generation.
4. Click `Stop`.
5. Confirm:
   - generation stops quickly,
   - input area exits typing state,
   - session history is retained (no reset/clear),
   - user can send a new message in the same chat.

## Backend Checks

1. Confirm `POST /api/chat/stop` returns `{ "status": "ok", ... }`.
2. Confirm manager receives stop command and forwards runtime control.
3. Confirm agent loop emits stop-related termination event path without crash.
