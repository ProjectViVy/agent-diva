# P1-5 Guardian Default Scope

## Summary

- Changed `GuardianConfig::default()` to keep `auto_approve_known_safe` and `enable_auto_learning` disabled by default.
- Preserved the explicit permissive behavior on `GuardianConfig::liberal()`.
- Added a regression test to ensure the default reviewer does not auto-approve a known-safe command when approval is required.

## Impact

- New Guardian users now start from a fail-closed default and must opt into permissive auto-approval behavior explicitly.
