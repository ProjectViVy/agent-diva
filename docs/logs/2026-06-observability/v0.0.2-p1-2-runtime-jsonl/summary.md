# Summary

## Scope

Implemented the first executable slice of Phase B observability and fixed the
`docs/dev/README.md` nano packaging dead link.

## Changes

- Added `agent-diva-core::trace` with typed `TraceId`, structured `TraceEvent`,
  and an append-only JSONL runtime log writer.
- Added redaction and truncation before structured runtime log persistence.
- Extended `LoggingConfig` with `structured_runtime_logs_enabled`,
  `retention_days`, `runtime_log_dir`, and `record_tool_output_summaries`.
- Wired `TraceLogger` into CLI, manager, and GUI-resolved logging paths.
- Emitted structured runtime events from `agent-diva-agent` for
  `message_received`, `llm_request_started`, `llm_response_completed`,
  `llm_response_failed`, `tool_call_started`, `tool_call_completed`,
  `tool_call_failed`, and `runtime_cancelled`.
- Replaced the broken `docs/dev/README.md` nano packaging link with the
  archived nano/shared-runtime index.

## Impact

`main` now has a usable, redacted, durable runtime observability base layer
without taking on debug bundle, gateway-wide event wiring, or GUI settings
scope in the same patch.
