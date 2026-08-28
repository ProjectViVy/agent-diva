# agent-diva-service

## OVERVIEW

Windows service companion (`agent-diva-service` binary). Registers as `AgentDivaGateway` service and spawns `agent-diva.exe gateway run` as a child process.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| CLI entry / non-Windows stub | `src/main.rs` |
| Windows service implementation | `src/main.rs` `windows_impl` module |
| Service control handler | `src/main.rs` (`service_main`, `run_service`) |
| Gateway child spawning | `src/main.rs` (`spawn_gateway_child`) |

## CONVENTIONS

- The service resolves the sibling `agent-diva.exe` next to its own executable.
- `--console` mode is for local validation only.
- Service lifecycle status is reported to the Windows SCM.

## ANTI-PATTERNS

- Do not run this crate on non-Windows platforms; it exits with an error.
- Do not add business logic here; it only wraps `agent-diva gateway run`.
- Do not leak service credentials or tokens in logs.

## NOTES

- CI dry-runs `service install/status/uninstall --dry-run` on Windows.
