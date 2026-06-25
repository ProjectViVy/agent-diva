# Acceptance

1. Build the workspace and run `cargo test`.
2. Trigger a normal agent turn that produces a tool call.
3. Confirm bus subscribers can observe:
   - `DecisionPoint`
   - `ToolInvoked`
   - `TokenUsed`
   - `PiiRedacted` when tool output contains secrets
4. Trigger heartbeat with actionable `HEARTBEAT.md` content and confirm `HeartbeatTriggered`.
5. Let presence go idle past threshold or force refresh in tests and confirm `PresenceChanged`.
