# P1-4 macOS sandbox-exec Fail-Open

## Summary

- Changed the macOS executor to return an explicit platform-unavailable error when `sandbox-exec` is missing.
- Stopped the implicit direct-execution fallback in the Seatbelt execution path.
- Updated sandbox escalation handling to treat the new unavailable error as approval-gated retryable context.

## Impact

- macOS callers now receive a hard sandbox failure signal instead of silently running commands without isolation.
