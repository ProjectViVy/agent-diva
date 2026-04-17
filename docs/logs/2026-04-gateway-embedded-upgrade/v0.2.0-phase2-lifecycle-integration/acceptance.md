# Acceptance

## User-Facing Checks

1. Start the GUI in release mode and confirm the embedded gateway starts automatically without waiting for a delayed subprocess launch.
2. Confirm the generated `gateway.port` matches the actual embedded runtime port used by GUI API requests.
3. With `closeToTray=false`, close the main window and verify the embedded gateway is shut down before the app exits.
4. With `closeToTray=true`, close the main window and verify the window hides while the embedded gateway remains running.
5. Use the tray `Quit` action and verify the app exits cleanly without leaving the embedded gateway active.
6. Open the existing gateway control UI and verify status polling still works through `get_gateway_process_status`.
7. Start the gateway manually in debug mode, then launch the GUI in debug mode and verify the GUI does not try to manage embedded lifecycle.
