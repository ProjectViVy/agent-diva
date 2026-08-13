# Acceptance

1. Start the GUI with `pnpm --dir agent-diva-gui tauri dev`.
2. Close the main window with `closeToTray = false` and verify the process exits cleanly.
3. Quit from tray menu and verify the process exits cleanly.
4. Open `desktop-pet`, then quit again and verify the shutdown still exits without the `Chrome_WidgetWin_0` unregister error.
