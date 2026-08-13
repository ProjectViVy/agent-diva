# Summary

- Added `agent-diva-gui/.taurignore` to exclude `src-tauri/gen/**` from the Tauri dev watcher.
- This prevents generated capability/schema files from being treated as Rust-side changes and re-triggering the `src-tauri/build.rs changed` rebuild loop during `pnpm tauri dev`.
- Scope is limited to Tauri dev-time file watching; runtime and bundle behavior are unchanged.
