# Summary

- Added startup timeout guards in `agent-diva-gui/src/App.vue` for splash-blocking initialization calls.
- Wrapped session list loading and startup session restore calls so a stalled local API can fail fast instead of trapping the GUI on the initialization screen.
- Kept startup behavior best-effort: configuration and tools still load when available, but failures now degrade into warnings rather than indefinite splash blocking.
