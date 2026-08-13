# Summary

- Added crate feature declarations for sandbox manager, orchestrator, platform, approval, guardian, and filesystem concerns.
- Applied `#[cfg(feature = ...)]` gates in `lib.rs` for module declarations and re-exports.
- Preserved the default crate surface while enabling explicit build profiles for future modularization.

# Impact

- Default builds remain compatible.
- The crate now exposes a first-pass feature boundary for follow-up API cleanup.
