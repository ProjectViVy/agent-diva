# Summary

- Added a topo-ordered nano stack publish helper script: `scripts/publish-nano-stack.ps1`.
- Added matching `just` recipes for package-check and publish dry-run flows.
- Unified `rust-version = "1.80.0"` across the nano publish closure crates.
- Verified that the helper stops at the first crate whose upstream dependency is not yet published on crates.io.

# Impact

- Publishing the nano stack no longer depends on remembering crate order manually.
- The repository now surfaces the real next blocker clearly: upstream crates must be published before higher-level crates can package/publish successfully.
