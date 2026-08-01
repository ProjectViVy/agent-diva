# Release

The focused source update is committed on the local `agent-diva-pro` branch. It is not pushed or deployed.

The Tauri rebuild produced updated `target/release/agent-diva-gui.exe`, NSIS,
and MSI artifacts. Windows then denied direct launch of the updated executable
with OS error 5. The previous GUI was stopped before linking and no GUI process
is currently running. Desktop deployment therefore remains blocked pending the
recorded local execution-policy diagnosis.
