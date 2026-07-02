# Summary

## What Changed

- Initialized GUI process logging before the Tauri builder starts.
- Resolved the GUI log directory with the same config-relative rules used by the log viewer.
- Kept the tracing non-blocking worker guard alive for the full GUI process lifetime.

## Impact

- Embedded gateway logs can be written to the configured file directory.
- Settings > General > Gateway Logs can read the same files created by the embedded gateway.
- CLI logging behavior is unchanged.

## Root Cause

The GUI embedded gateway bypassed the CLI entrypoint, so it never called the shared tracing initialization used by `agent-diva-cli`. The log viewer also reads config-relative log paths, so GUI logging must initialize with the same resolved directory.
