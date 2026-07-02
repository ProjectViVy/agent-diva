# Summary

- Added `SandboxPolicy::to_security_policy(workspace_dir)` as a forward bridge into `agent-diva-core::security::SecurityPolicy`.
- Added `SecurityPolicySandboxExt::to_sandbox_policy()` as the reverse bridge without creating a `core -> sandbox` dependency cycle.
- Documented the mapping table and added conversion tests in `policy.rs`.

# Impact

- Sandbox and core security types now have an explicit compatibility bridge.
- The conversion is documented as intentionally lossy where the two models do not line up exactly.
