# Verification

- Ran `cargo fmt`
  - Result: success
- Ran `cargo test`
  - First attempt hit local timeout at 124s due workspace size.
- Re-ran `cargo test -- --nocapture`
  - Result: success
  - Observed: workspace tests passed; existing unrelated warnings remain in `agent-diva-channels` / `agent-diva-tools` tests.

# Notes

- Added explicit assertions for `DecisionPoint`, `TokenUsed`, `ToolInvoked`, `PiiRedacted`, `InjectionDetected`, `PresenceChanged`, and `HeartbeatTriggered`.
