# v0.0.1 P0-3 Credential Scrubbing

## Summary

This iteration closes `P0-3: Credential scrubbing in logs`.

- Added `agent-diva-core::redaction` with shared secret scrubbing helpers.
- Wrapped runtime stdout/file tracing writers with a redacting writer so text and JSON logs are scrubbed before output.
- Hardened `ErrorContext` to redact content and metadata before formatting.
- Replaced manager config update `Debug` logging with a safe summary that reports `has_api_key` instead of raw credentials.

## Impact

- Runtime logs are safer across CLI, GUI, and manager paths that use `agent_diva_core::logging`.
- Existing CLI `config show` redaction remains intact.
- No config schema, provider request payload, or public API contract changed.
