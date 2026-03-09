# Acceptance

## Checks
1. Trigger delete session from GUI/API.
2. Confirm manager log contains `DeleteSession completed` with `session_key` and `deleted`.
3. Confirm runtime log contains `Runtime delete session completed` on success.
4. If runtime deletion fails, confirm `Runtime delete session failed` appears with error context.
