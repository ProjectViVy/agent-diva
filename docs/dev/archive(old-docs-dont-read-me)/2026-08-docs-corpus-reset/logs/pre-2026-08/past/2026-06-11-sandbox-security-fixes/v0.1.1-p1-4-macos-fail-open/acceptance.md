# Acceptance

1. Run sandboxed execution on macOS where `sandbox-exec` is unavailable.
2. Confirm the executor returns `SandboxError::PlatformUnavailable`.
3. Confirm no direct unsandboxed execution occurs unless a higher layer explicitly retries after approval.
