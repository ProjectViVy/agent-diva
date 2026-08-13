# Release

- No special deployment step is required beyond the normal Rust workspace build and GUI packaging flow.
- Manager API and Tauri GUI both consume the same runtime discovery backend, so no data migration is needed.
- If this change is released separately from the GUI, the CLI and manager endpoint remain independently usable.
