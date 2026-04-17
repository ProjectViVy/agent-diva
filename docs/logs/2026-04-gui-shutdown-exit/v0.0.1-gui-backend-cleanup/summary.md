# Iteration Summary

## Change

This iteration fixes the GUI shutdown path in `agent-diva-gui` so closing the main window or using tray `Quit` no longer relies on `std::process::exit(0)` while background work is still running.

## Impact

- Added a shared shutdown manager with a cancellation token for GUI-managed background tasks.
- Reworked full-exit flow to cancel background work, stop the embedded gateway with timeout protection, and then exit through Tauri lifecycle APIs.
- Made the background event stream task respond to shutdown cancellation instead of looping forever.

## User-visible Result

When `closeToTray=false`, clicking the main window close button now performs a coordinated full shutdown instead of leaving backend activity behind. Tray `Quit` follows the same shutdown path.
