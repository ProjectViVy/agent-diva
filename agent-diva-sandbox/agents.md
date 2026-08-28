# agent-diva-sandbox

## OVERVIEW

Process isolation and command approval for shell execution. Combines sandbox policy, platform-specific executors, rule-based `ExecPolicy`, `Guardian` auto-approval, and `ToolOrchestrator` execution flow.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| Sandbox policy types | `src/policy.rs` (`SandboxPolicy`, `SandboxMode`) |
| Sandbox manager / executor entry | `src/manager.rs` (`SandboxManager`, `SandboxConfig`, `SandboxExecRequest`) |
| Command rule evaluation | `src/command_rules.rs`, `src/rules.rs` |
| ExecPolicy manager | `src/exec_policy.rs` (`ExecPolicyManager`, `BANNED_PREFIX_SUGGESTIONS`) |
| Guardian auto-approval | `src/guardian.rs` (`GuardianManager`, `GuardianConfig`) |
| Approval store / coordinator | `src/approval.rs`, `src/approval_coordinator.rs` |
| Tool orchestrator | `src/orchestrator.rs` (`ToolOrchestrator`) |
| Platform executors | `src/platform.rs` / platform-specific modules |
| Filesystem policy | `src/filesystem.rs` (`FileSystemSandboxPolicy`, `WritableRoot`) |
| Governance adapter | `src/governance_adapter.rs` |
| Disable flag | `src/lib.rs` (`AGENT_DIVA_SANDBOX_DISABLED`) |

## CONVENTIONS

- New sandbox modes extend `SandboxMode`/`SandboxPolicy`; platform-specific logic stays in `platform/`.
- New approval rules extend `CommandRule` and are evaluated by `ExecPolicyManager`.
- Feature-gate heavy modules: `manager`, `orchestrator`, `platform`, `approval`, `guardian`, `filesystem`. Defaults include `manager` + `platform`.

## ANTI-PATTERNS

- **Never** auto-suggest a banned prefix (python, bash, sudo, pwsh, node, etc.) as an Allow rule.
- Do not execute commands with `DangerFullAccess` unless explicitly approved.
- Do not store approval decisions without scoping them to session/command/cwd.
- Do not add platform-specific code in `policy.rs` or `rules.rs`.

## NOTES

- Set `AGENT_DIVA_SANDBOX_DISABLED=1` to disable sandbox entirely.
- `just feature-gate-check` verifies each non-default feature combination compiles.
