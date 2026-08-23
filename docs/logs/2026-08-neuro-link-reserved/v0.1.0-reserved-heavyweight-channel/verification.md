# Verification

- Comments land on `schema.rs`, `cli_runtime.rs` `channel_statuses`, and
  `neuro_link.rs`.
- `channel_statuses` still does not emit a `neuro-link` row.
- `rustfmt --edition 2021` on the three Rust files: clean.
- No behavior tests: comments/TODOLIST only; `channel_statuses` still
  omits `neuro-link`.
