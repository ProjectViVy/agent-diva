# Acceptance

## Manual Acceptance Steps

1. Launch the GUI on Windows.
2. Ensure `closeToTray` is disabled.
3. Close the main window.
4. Confirm the GUI exits fully and no lingering GUI/backend process remains.
5. Launch the GUI again and use tray `Quit`.
6. Confirm the GUI exits fully and no lingering GUI/backend process remains.
7. Enable `closeToTray`.
8. Close the main window.
9. Confirm the window hides to tray and the app does not exit.

## Expected Outcome

- Full exit paths shut down the embedded gateway and stop background stream tasks.
- Tray residency remains unchanged when the user explicitly enabled it.
