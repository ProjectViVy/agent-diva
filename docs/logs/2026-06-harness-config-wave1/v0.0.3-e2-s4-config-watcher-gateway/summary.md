# E2-S4 Config Watcher Gateway

## Scope

- Wired `ConfigWatcher` into gateway runtime startup and shutdown.
- Added a tooling hot-reload bridge so gateway modules receive `on_config_reload` callbacks from watcher events.
- Made watcher restart-required diffs warn and skip live apply instead of silently mutating in-memory config.
- Added live-reload coverage for presence and heartbeat runtime config updates.

## Impact

- Wave 1 config-line work now has an actual gateway-side watcher loop instead of core-only hot-reload primitives.
- Invalid watched config files preserve the prior live runtime state and emit explicit reload-failure logs.
- Restart-required edits now stay restart-required at runtime and no longer masquerade as hot reload.
