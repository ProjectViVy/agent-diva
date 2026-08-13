# Acceptance

## User-Facing Acceptance Steps

- Confirm `agent-diva-core/src/audit_sink.rs` contains `AuditSink` trait and `JsonlAuditSink`.
- Confirm `agent-diva-providers/src/tap.rs` contains `ProviderTap<P>`.
- Confirm `agent-diva-tooling/src/registry.rs` emits `ToolExecuted` audit events.
- Confirm `agent-diva-manager/src/handlers/logs.rs` provides `GET /api/logs`.
- Confirm `agent-diva-core/src/rate_limiter.rs` contains `TokenBucket`.
- Confirm `agent-diva-agent/src/compaction/meta.rs` contains `MetaCompactor`.
- Confirm workspace-root `AGENTS.md` lists Wave 7 as ✅ Done.

## Acceptance Result

Accepted when all code and documentation checks above pass.
