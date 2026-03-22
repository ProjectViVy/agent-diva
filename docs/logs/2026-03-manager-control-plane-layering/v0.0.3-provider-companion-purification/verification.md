# Verification

- No validation commands were run in this iteration.
- `cargo check`, `cargo test`, GUI build, and smoke tests were intentionally skipped to prioritize fast dirty-worktree convergence.

# Result

- Status: not verified.
- The change should be treated as structural refactoring pending later compile and behavior validation.
- Main remaining risk areas:
  - missing module imports or exports
  - visibility issues after moving methods into dedicated modules
  - handler wiring mistakes that would only surface during compile or route exercise
