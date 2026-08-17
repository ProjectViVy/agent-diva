# Acceptance

## User-visible scenarios

1. Open a conversation containing enough content to make the chat list scroll.
2. While an assistant response is streaming, scroll upward to an older message.
3. Confirm new chunks and inline question cards do not change the reading
   position.
4. Scroll to within 48 px of the bottom and confirm later chunks remain visible.
5. Scroll upward again, send a new message, and confirm the list returns to the
   latest message.
6. Switch to another conversation and confirm it opens at its latest content.

## Result

- Long-message upward-reading scenario: accepted by the user on 2026-08-17.
- Bottom resume, send, session switch, and inline-card cases: covered by
  deterministic component tests.
