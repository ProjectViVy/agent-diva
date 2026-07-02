# Summary

## Changes

- Finalized the GUI embedded-gateway rollout by explicitly demoting legacy external gateway lifecycle helpers to a compatibility layer.
- Marked legacy gateway process helpers in `process_utils.rs` as deprecated and documented them as maintenance/debug utilities rather than release-mode lifecycle primitives.
- Converted Tauri commands `start_gateway`, `stop_gateway`, and `uninstall_gateway` into deprecated compatibility wrappers with embedded-mode messaging.
- Reduced `wipe_local_data` to a best-effort cleanup path for stray legacy gateway processes instead of relying on the old external-process management model.
- Updated architecture documentation to describe release-mode embedded gateway behavior, debug-mode external gateway expectations, and the retained compatibility layer.

## Impact

- Release builds now present a clearer embedded-first contract across command handlers and maintenance flows.
- Existing frontend or automation callers that still invoke the legacy gateway commands do not hard-fail due to missing commands, but they now receive explicit compatibility behavior.
- Gateway cleanup responsibilities are narrower and easier to reason about: normal lifecycle stays in-process, while destructive maintenance performs best-effort legacy cleanup only.
