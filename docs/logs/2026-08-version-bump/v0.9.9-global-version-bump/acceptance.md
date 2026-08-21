# Acceptance: Global Version Bump 0.5.0 -> 0.9.9

## Steps

1. In the workspace root, run `cargo metadata --format-version 1 --no-deps`
   and confirm every workspace member reports version `0.9.9`.
2. `grep -rn '"0.5.0"' --include=Cargo.toml` at the workspace root returns
   nothing outside `.workspace/` and `target/`.
3. Optional: after a rebuild, `just run -- --version` (or the CLI's version
   output) should report `0.9.9`.
4. Optional: the next packaging run produces installers whose filenames and
   metadata carry `0.9.9`.
