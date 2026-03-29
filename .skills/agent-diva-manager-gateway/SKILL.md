---
name: agent-diva-manager-gateway
description: Local HTTP gateway and control plane for agent-diva-cli via agent-diva-manager (Axum). Use when adding or changing REST routes, handlers, AppState, ManagerCommand wiring, CORS/tracing layers, or CLI gateway startup (run_local_gateway, GatewayRuntimeConfig). Covers server.rs route groups and handler modules.
---

# Manager gateway (`agent-diva-manager`)

## Role

`agent-diva-manager` is the **default local gateway** for **`agent-diva-cli`**: HTTP API for chat, sessions, config, channels, tools, providers, cron, MCP, skills, SSE events, and health. The CLI command **`agent-diva gateway run`** (see `agent-diva-cli/src/main.rs`) calls **`run_local_gateway`** with **`GatewayRuntimeConfig`**.

Public entry points: `agent-diva-manager/src/lib.rs` exports `run_local_gateway`, `GatewayRuntimeConfig`, `DEFAULT_GATEWAY_PORT`, `run_server`, `AppState`, `ManagerCommand`, etc.

## Layout

| Area | Path | Notes |
|------|------|--------|
| App + shutdown | `src/server.rs` | `build_app`, `run_server` (bind `127.0.0.1`, graceful shutdown via `broadcast`) |
| Runtime API | `server.rs` → `runtime_routes()` | `/api/chat`, `/api/chat/stop`, `/api/events`, `/api/sessions`, `/api/config`, `/api/channels`, `/api/tools`, `/api/skills`, `/api/mcps`, `/api/cron/jobs`, … |
| Providers API | `provider_routes()` | `/api/providers`, `/api/providers/resolve`, `/api/providers/:name`, models subroutes |
| Health | `misc_routes()` | `/api/health` |
| Handlers | `src/handlers/*.rs` | Keep thin; delegate to `AppState` / `Manager` / bus |
| Shared state | `src/state.rs` | `AppState` (e.g. `api_tx` for manager commands, `MessageBus`) |
| Composition | `src/manager.rs`, `src/runtime.rs` | Gateway lifecycle and command dispatch |

Middleware stack (see `build_app`): `CorsLayer::permissive()`, `TraceLayer::new_for_http()`.

## Adding or changing an API

1. Implement or extend a handler in `src/handlers/` using Axum extractors and `State<AppState>`.
2. Register the path in `server.rs` inside the correct `*_routes()` helper so related routes stay grouped.
3. If the handler needs new async services or channels, extend **`AppState`** and construct them where the gateway is built (`runtime.rs` / manager startup), not inside the handler module as globals.
4. **Internal command pattern:** many handlers send work on an **`mpsc`** sender with **`ManagerCommand`** variants; the test `build_app_keeps_health_and_skills_routes_without_overlap` in `server.rs` shows that requests can **hang** if nothing consumes commands—when adding similar flows, ensure the runtime always has a task reading the channel or use `oneshot` replies correctly.
5. Return consistent HTTP status codes and JSON error bodies matching neighboring handlers.

## CLI integration

- **`agent-diva-cli`:** `run_gateway` → `run_local_gateway(build_gateway_runtime_config(...))`.
- Changing default port or bind address: trace `GatewayRuntimeConfig` and `DEFAULT_GATEWAY_PORT`.

## Validation

- `cargo test -p agent-diva-manager` (includes Axum route smoke tests in `server.rs`).
- Manual smoke: `just run -- gateway run` (or your local equivalent), then `GET /api/health`; verify clean shutdown (Ctrl+C) without panic.

## Related skills

- **`agent-diva-rust-dev`**: Axum/async/error-handling conventions for this workspace.
- **`agent-diva-workspace-validate`**: full workspace CI commands.
- **`agent-diva-extend-integrations`**: when a new API exposes providers/channels/tools/config fields you just added elsewhere.
- **`agent-diva-core-data-flow`**: how HTTP chat relates to the message bus and agent (conceptual).
