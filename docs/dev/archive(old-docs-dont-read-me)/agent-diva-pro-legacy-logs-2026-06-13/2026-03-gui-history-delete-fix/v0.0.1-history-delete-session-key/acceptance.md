# Acceptance

## User-facing checks
1. Open GUI history dropdown and delete a session: target item disappears immediately.
2. Delete currently loaded session: chat view resets to cleared state.
3. If backend deletion fails, session is still removed locally in current run and an error is logged.
4. Re-open history dropdown after refresh: deleted item does not immediately reappear in current runtime.

## Notes
- Full persistence after restart depends on backend delete success.
