# Acceptance

1. Run `pnpm --dir agent-diva-gui tauri dev`.
2. Wait for the GUI window to open normally.
3. Close the GUI.
4. Confirm the final CLI output does not re-enter `Info File src-tauri\\build.rs changed. Rebuilding application...`.
5. Confirm no unexpected second `cargo run` is started during shutdown.
