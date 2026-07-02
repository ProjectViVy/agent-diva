# Release

## Method

No standalone release executed in this iteration.

## Suggested rollout

1. Deploy updated gateway (manager + agent + channels).
2. Deploy updated GUI/Tauri app.
3. Smoke-check `/stop` in:
   - GUI text input
   - TUI local mode
   - TUI remote mode
   - Telegram command path
