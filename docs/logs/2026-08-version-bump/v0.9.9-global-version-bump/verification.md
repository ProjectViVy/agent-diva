# Verification: Global Version Bump 0.5.0 -> 0.9.9

## Commands and results

1. Bulk sed replacement of `"0.5.0"` -> `"0.9.9"` across root workspace
   `Cargo.toml` files (18 manifests; `.workspace/` and `target/` excluded).
2. Residual check: `grep -rn '"0\.5\.0"' --include=Cargo.toml` (workspace
   scope) -> no matches.
3. `cargo metadata --format-version 1 --no-deps` -> resolution succeeded;
   `Cargo.lock` refreshed, `agent-diva-core` now `0.9.9`, 17 lock entries at
   `0.9.9` (one per workspace member crate).
4. `cargo check --workspace` -> see results recorded below when the run
   completes (validation gate for this metadata-only change; version fields
   do not alter code, so a clean resolution + check is the acceptance bar).

## Not applicable

- `just fmt-check`: no Rust source files changed (Cargo.toml is outside
  rustfmt scope).
- GUI vitest / vue-tsc: no frontend source changed; npm package version
  intentionally untouched.
