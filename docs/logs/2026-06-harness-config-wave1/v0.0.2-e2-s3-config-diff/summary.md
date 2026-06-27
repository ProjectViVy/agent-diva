# E2-S3 Config Diff

## Scope

- Added core `ReloadPlan` / `ConfigDiff` support for classifying hot-reloadable versus restart-required config paths.
- Added `agent-diva config diff --new-config <path> --format json|pretty` as the observable CLI surface for Wave 1 config diff QA.
- Updated hot-reload field detection to reuse the same classification source of truth.

## Impact

- Wave 1 config-line work now has a real pre-reload planning surface instead of implicit watcher-only behavior.
- CLI and core tests cover hot-reload, invalid-candidate, noop, and restart-required scenarios.
