# Summary

## Changes

- Switched `agent-diva-gui` release-mode gateway startup from spawned CLI subprocess management to embedded runtime startup during Tauri setup.
- Added managed embedded gateway state and serializable gateway runtime status for command compatibility.
- Unified main-window close and tray quit paths to gracefully shut down the embedded gateway before exiting.
- Updated GUI runtime state wiring so manager API requests use the actual embedded random port selected at startup.
- Kept legacy gateway control commands callable for frontend compatibility, but redirected them to embedded-mode semantics.

## Impact

- Release builds no longer depend on delayed subprocess startup or orphan-process cleanup.
- Debug builds still assume an externally started gateway.
- Existing frontend `get_gateway_process_status` consumers remain functional without requiring immediate Vue-side refactors.
