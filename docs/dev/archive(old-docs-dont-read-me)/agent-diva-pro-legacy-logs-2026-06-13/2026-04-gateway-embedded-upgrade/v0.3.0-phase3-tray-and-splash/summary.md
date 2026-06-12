# Summary

## Changes

- Expanded the Tauri tray menu from a minimal `Show` / `Quit` pair to a richer release-mode control surface with gateway status, config-directory access, and logs-directory access.
- Added tray status text refresh so embedded gateway shutdown paths update the menu from running to stopped.
- Reused the existing config loader and log-directory resolution rules for tray directory-open actions instead of introducing a second path policy.
- Simplified splash completion by removing the fixed backend delay; release-mode setup now marks backend initialization complete immediately after embedded gateway startup and port persistence succeed.
- Updated gateway status formatting to a tray-friendly user-visible string and aligned the GUI test coverage with the new wording.

## Impact

- Release builds expose the embedded gateway state directly in the system tray.
- Tray quit and normal shutdown paths now keep the tray status text consistent with embedded gateway lifecycle changes.
- Splash dismissal is no longer tied to an arbitrary delay once the embedded gateway startup sequence has completed.
- No public config schema, CLI contract, or manager API surface was added in this phase.
