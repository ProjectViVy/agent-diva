# Summary

- Fixed the Windows GUI shutdown path to close all Tauri webview windows before calling `app.exit(0)`.
- Stopped intercepting `CloseRequested` once shutdown has already started, so programmatic window closes can complete cleanly.
- Target symptom: `Failed to unregister class Chrome_WidgetWin_0. Error = 1412` during `tauri dev` shutdown on Windows.
