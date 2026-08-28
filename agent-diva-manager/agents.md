# agent-diva-manager

## OVERVIEW

Local gateway runtime and HTTP control plane. Exposes the Axum server, route handlers, runtime lifecycle, and manager state consumed by `agent-diva-cli` and the GUI.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| Gateway runtime entry | `src/lib.rs`, `src/runtime.rs` (`run_local_gateway`, `start_embedded_gateway_runtime`, `GatewayRuntimeConfig`) |
| HTTP server / router | `src/server.rs` (`build_router`, `run_server`) |
| HTTP handlers | `src/handlers/` |
| Manager state | `src/state.rs` (`AppState`, `ManagerCommand`) |
| Approval service | `src/approval_service.rs` |
| File service | `src/file_service.rs` |
| Planning service | `src/planning_service.rs` |
| Skill service | `src/skill_service.rs` |
| MCP service | `src/mcp_service.rs` |
| Marketplace | `src/marketplace.rs` |

## CONVENTIONS

- This crate is a library; the CLI calls `run_local_gateway()` to start the gateway.
- New HTTP routes are added in `src/server.rs` and implemented in `src/handlers/`.
- Handlers receive `State<AppState>` and interact with services through it.
- The embedded runtime is used by the GUI; the standalone runtime is used by `agent-diva gateway run`.

## ANTI-PATTERNS

- Do not put CLI concerns (argument parsing, terminal UI) here.
- Do not start a gateway directly from unit tests; use test fixtures or mocks.
- Do not bypass `AppState` to reach global singletons.

## NOTES

- `just health-benchmark-check` runs the ignored `health_benchmark_ci_gate_stays_within_budget` test here.
- Recovery drills (`just e7-recovery-drills`) include manager-level journal recovery tests.
