# P3-12 Pro Clippy Fixes

## Summary

- Removed an unnecessary `mut` binding in `agent-diva-files`.
- Replaced manual `Default` implementations with derived defaults for config enums in `agent-diva-core`.

## Impact

- Clears targeted clippy warnings without changing runtime behavior.
- Keeps existing default variants unchanged for sandbox and mask configuration.
