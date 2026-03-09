# Iteration Summary

## Changes
- Fixed GUI history delete flow to use full `session_key` (`channel:chat_id`) end-to-end instead of only `chat_id`.
- Updated history dropdown interactions to emit `session_key` for both load and delete actions.
- Added local deleted-session filtering to prevent immediate visual reappearance when backend deletion fails.
- Kept optimistic UI behavior: even if backend deletion fails, session is removed from the current frontend runtime.

## Impact
- User-visible behavior: history delete action now removes the intended session more reliably.
- Scope: `agent-diva-gui` frontend only (`App.vue`, `NormalMode.vue`).
- Backward compatibility: existing manager API compatibility is preserved; frontend now passes explicit full keys.
