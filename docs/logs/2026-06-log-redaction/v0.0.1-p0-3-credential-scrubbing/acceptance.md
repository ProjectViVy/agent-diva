# Acceptance

1. Start any runtime path that initializes `agent_diva_core::logging`.
2. Emit log content containing a bearer token, `sk-*`, `ghp_*`, or an `api_key` field.
3. Verify stdout and log files show `***REDACTED***` instead of the raw secret.
4. Trigger manager config update logging and verify only the safe summary is emitted (`provider`, `model`, `api_base`, `has_api_key`).
5. Run `agent-diva config show --format json` and verify secret fields remain redacted.
