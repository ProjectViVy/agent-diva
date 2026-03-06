# Summary

## Scope

Implemented a first end-to-end stop-interrupt flow for GUI chat generation:

- GUI chat input area now includes a `Stop` button placed left to `Send`.
- Tauri now exposes `stop_generation` command to request stop from backend API.
- Manager API now exposes `POST /api/chat/stop`.
- Manager forwards stop requests to the runtime control channel.
- Agent loop supports stop commands and performs cooperative cancellation checks while generating.

## Key Design

- Stop semantics are `stop-only` (interrupt current generation, keep session history).
- Session reset behavior remains separate and unchanged.
- Supports local/remote backend style through the same GUI call path (`GUI -> Tauri -> API`).

## Files Changed

- `agent-diva-gui/src/components/ChatView.vue`
- `agent-diva-gui/src/components/NormalMode.vue`
- `agent-diva-gui/src/App.vue`
- `agent-diva-gui/src/locales/en.ts`
- `agent-diva-gui/src/locales/zh.ts`
- `agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva-gui/src-tauri/src/lib.rs`
- `agent-diva-manager/src/server.rs`
- `agent-diva-manager/src/handlers.rs`
- `agent-diva-manager/src/state.rs`
- `agent-diva-manager/src/manager.rs`
- `agent-diva-agent/src/runtime_control.rs`
- `agent-diva-agent/src/agent_loop.rs`
