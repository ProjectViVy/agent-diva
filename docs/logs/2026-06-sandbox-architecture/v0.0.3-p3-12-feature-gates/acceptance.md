# Acceptance

1. `agent-diva-sandbox` declares the requested feature names in `Cargo.toml`.
2. `lib.rs` gates module declarations and re-exports with `#[cfg(feature = ...)]`.
3. Default, reduced, and all-feature build profiles all compile.
