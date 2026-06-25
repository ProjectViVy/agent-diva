# Summary

- Added `agent-diva-core::audit` with a structured `AuditEvent` enum, `AuditLogger`, daily `gateway.log.YYYY-MM-DD` naming helper, and shared log cleanup matching.
- Wired `MessageBus::emit()` to mirror audit-related `AgentBusEvent` values into structured `tracing` JSON so agent-loop tool calls, decision points, injection detections, presence changes, token usage, and heartbeat events land in the daily gateway log automatically.
- Added Tauri audit log readers plus a new Settings -> Audit page in the GUI that supports date selection, structured event browsing, and raw log inspection.
- Added Rust tests for audit emission/parsing and updated the GUI Tauri crate dependency set to include `chrono` for date parsing.

## Impact

- Operators can now review behavioral audit events without reading mixed gateway logs by hand.
- Existing runtime log rotation/cleanup keeps handling audit files because they share the `gateway.log.*` naming pattern.
- The GUI audit page reads the same structured source of truth that the gateway writes, avoiding a second persistence path.
