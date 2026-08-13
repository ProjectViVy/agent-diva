# Acceptance

1. `ToolOrchestrator` no longer locks `SandboxManager`'s approval store directly.
2. Approval cache reads and one-time approval consumption go through manager-owned methods.
3. `cargo test -p agent-diva-sandbox` passes without approval/orchestrator regressions.
