# Verification

Worktree: `C:\Users\Administrator\Desktop\morediva\agent-diva-bml-governed-seam`
Branch: `chore/bml-governed-seam-dead-code` @ base `1471f1e6`

## Scan

`put_governed` / `rollback_governed` / `GovernedMemoryApply` in `*.rs`:

- `agent-diva-laputa/tests/bml_boundary_guard.rs` — `.put_governed(` / `.rollback_governed(` kept as anti-reintroduction needles
- `agent-diva-laputa/src/bml/mod.rs` — boundary note that the APIs are deleted

No other Rust call sites or type definitions remain.

`CREATE TABLE IF NOT EXISTS memory_apply_journal` still present in
`typed_store.rs`. `SCHEMA_VERSION` remains `1`.

## Gates (all green)

| Command | Result |
| --- | --- |
| `python scripts/ci/check_cognitive_clean_break.py --self-test` | `self-test passed` |
| `python scripts/ci/check_cognitive_clean_break.py` | `D4 §3.1 families absent from production paths` |
| `cargo test -p agent-diva-laputa` | 50 passed, 1 ignored (`ten_thousand_record_top_eight_search_p95_is_below_200ms`), 0 failed. Includes `governance_modules_must_not_write_bml_directly` |
| `cargo clippy -p agent-diva-laputa -- -D warnings` | green (lib target) |
| `just fmt-check` | green |

Pre-existing test-harness `dead_code` warnings in
`authority_boundary_guard.rs` still fire under `--tests`; they are
`LAPUTA-TESTS-1.94-ALL-TARGETS-CLIPPY`, not this slice.

## Skipped

- Full `just ci` / `just test`: laputa-only dead-API deletion; no GUI,
  Manager, or CLI behavior change.
- Desktop smoke: no user-visible path.
