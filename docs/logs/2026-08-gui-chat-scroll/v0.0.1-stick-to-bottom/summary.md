# GUI chat stick-to-bottom fix

## Summary

- Replaced unconditional message-end scrolling with scroll ownership on the
  `.chat-list` container.
- Streaming message, governance-card, and `ask_user` updates follow the latest
  content only while the user remains within 48 px of the bottom.
- Sending a message and switching or creating a session restore bottom
  following explicitly.
- Removed the obsolete end-of-list scroll anchor.

## Impact

Users can read older content during a long or streaming response without the
viewport being forced back to the latest message. Existing initial-open and
active-conversation bottom-follow behavior remains intact.
