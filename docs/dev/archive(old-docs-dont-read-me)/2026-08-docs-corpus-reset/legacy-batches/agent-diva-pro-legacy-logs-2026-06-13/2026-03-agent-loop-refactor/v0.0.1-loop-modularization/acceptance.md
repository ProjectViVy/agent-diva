# Acceptance

## Acceptance Steps
1. Confirm public exports still work from `agent-diva-agent` (`AgentLoop`, `ToolConfig`, `RuntimeControlCommand`).
2. Confirm AgentLoop run/direct APIs compile and run without signature changes.
3. Confirm `cargo clippy --all -- -D warnings` passes.
4. Confirm `cargo test -p agent-diva-agent` passes fully.
5. Spot-check that cron-triggered protections and runtime-control behavior remain unchanged in code paths under `loop_turn` and `loop_runtime_control`.

## Acceptance Result
- Completed for crate-level refactor scope.
- Workspace-wide test run blocked by external file lock (`agent-diva.exe`).
