# P0-1 Windows Sandbox Minimum Implementation

## Changed

- Enabled required Windows API features for Job Object, pipes, and file reads.
- Implemented Windows restricted-token sandbox execution with a kill-on-close Job Object.
- Added stdout/stderr capture through inherited pipes.
- Updated Windows sandbox availability for restricted/elevated levels when restricted token creation succeeds.

## Impact

Windows sandboxed command execution no longer fails immediately as unimplemented for the configured restricted-token path.
