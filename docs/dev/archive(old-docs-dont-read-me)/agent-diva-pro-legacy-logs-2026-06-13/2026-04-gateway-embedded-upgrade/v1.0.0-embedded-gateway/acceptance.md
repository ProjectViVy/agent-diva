# Acceptance

## User-Facing Checks

1. Start the GUI in release mode and confirm the embedded gateway starts automatically without invoking any external gateway management action.
2. Confirm the tray menu still shows the gateway status line and that the status reflects the embedded runtime port.
3. Invoke any frontend path that still calls `start_gateway`, `stop_gateway`, or `uninstall_gateway` and verify the app returns compatibility behavior instead of trying to manage the normal embedded lifecycle.
4. Trigger `wipe_local_data` and verify the app shuts down the embedded gateway first, then performs best-effort cleanup of stray legacy gateway processes before deleting local data.
5. Close the window with `closeToTray=true` and verify the main window hides while the embedded gateway remains available.
6. Use tray `Quit` or normal shutdown with `closeToTray=false` and verify the embedded gateway exits cleanly with no obvious lingering GUI-owned process.
