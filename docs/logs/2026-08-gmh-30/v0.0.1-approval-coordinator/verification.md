# Verification

- `cargo test -p agent-diva-core governance::coordinator --lib`
- `cargo test -p agent-diva-core governance --lib`
- `cargo clippy -p agent-diva-core --all-targets -- -D warnings`
- `just fmt-check`
- `just check`
- `just test`

Results on 2026-08-03:

- Coordinator tests: 4 passed.
- Full core governance tests: 32 passed.
- Core library Clippy with warnings denied: passed.
- `just fmt-check`: passed.
- `just check`: passed.
- First `just test`: one unrelated `logs_filter_by_range` parallel-suite failure;
  its focused rerun passed. The immediate full `just test` rerun passed.
- The optional core `--all-targets` Clippy probe exposed pre-existing test lint
  debt recorded in `TODOLIST.md`; it is not part of the repository `just check`
  recipe and does not affect the production library target.
