# Acceptance

## User-Facing Checks

1. Start the GUI in release mode and confirm the tray menu contains `Show Window`, a gateway status line, `Open Config Directory`, `Open Logs Directory`, and `Quit`.
2. Confirm the tray status line initially shows the embedded gateway as running with the selected port.
3. Click `Open Config Directory` and verify the system file explorer opens the configured Agent Diva config directory.
4. Click `Open Logs Directory` and verify the system file explorer opens the resolved logging directory when it exists; if it does not exist, the app should log a warning and stay running.
5. With `closeToTray=true`, close the main window and verify the window hides while the tray status still shows the gateway as running.
6. Use tray `Quit` or normal shutdown with `closeToTray=false` and verify the embedded gateway stops and the tray status is updated to stopped during shutdown.
7. Launch the GUI in release mode and verify splash dismissal is no longer gated by a fixed backend sleep after embedded gateway startup succeeds.
