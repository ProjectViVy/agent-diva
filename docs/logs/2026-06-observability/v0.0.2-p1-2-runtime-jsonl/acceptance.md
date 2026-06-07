# Acceptance

1. Confirm `docs/dev/README.md` no longer points to the missing
   `nano-runtime-packaging-plan.md`.
2. Confirm `agent-diva-core/src/trace/` exists and defines `TraceId`,
   `TraceEvent`, and a JSONL runtime writer.
3. Confirm the new logging config fields exist and validate with sane defaults.
4. Confirm one runtime turn can persist correlated structured events under a
   single `trace_id`.
5. Confirm secrets and oversized text are redacted/truncated before JSONL
   persistence.
6. Confirm existing tracing logs continue to coexist with the new runtime JSONL
   channel.
