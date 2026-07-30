# Verification

- `cargo test -p agent-diva-migration`: 3 passed.
- `cargo clippy -p agent-diva-migration -- -D warnings`: passed.
- Coverage asserts apply and replay report identical before/after revisions,
  rollback restores revision zero, and reapply succeeds without replacing the
  verified backup.
