# Release

Available recipes:

- `just e7-recovery-drills`
- `just e7-vertical-e2e`
- `just gui-automated-check`
- `just e7-automated-release-gate`

The aggregate gate includes formatting, clippy, all Rust tests, health
benchmark, feature gates, Laputa clean-break, recovery, vertical E2E, GUI tests
and build, and Tauri check. It never reads desktop keys or invokes a provider.
