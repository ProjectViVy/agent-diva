# Token Statistics UI and API Fixes

## What was changed
1. Fixed `TokenInMemoryStats` mapping in `agent-diva-gui/src-tauri/src/commands.rs` and `InMemoryStats` in `agent-diva-gui/src/api/tokenStats.ts`. Previously, the GUI expected `input_tokens` and `output_tokens`, but the Manager API provides `total_input` and `total_output`. This schema mismatch caused Serde deserialization to fail for realtime token usage.
2. Removed silent `.catch(() => null)` handlers in `TokenStatsPanel.vue`'s `Promise.all` logic. Previously, if the Manager API was unreachable or returned an error, the errors were silently swallowed, returning `null` for `totalRes`. This caused the token stats component to render an entirely blank panel without showing the actual connection or parsing error to the user.

## Impact range
- `agent-diva-gui/src-tauri/src/commands.rs` (Tauri API schema)
- `agent-diva-gui/src/api/tokenStats.ts` (Frontend API schema)
- `agent-diva-gui/src/components/console/TokenStatsPanel.vue` (Frontend rendering logic)
