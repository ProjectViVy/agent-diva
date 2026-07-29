# Verification

## Focused checks

- `cargo test -p agent-diva-gui embedded_server::tests -- --nocapture`: passed, 5 tests.
- `pnpm test -- src/locales/evolution.test.ts src/components/NormalMode.test.ts`: passed, 2 files / 8 tests.
- `cargo test -p agent-diva-core supervised::executor::tests::test_executor_fails_when_no_handler`: passed.
- `cargo test -p agent-diva-core supervised::executor::tests::test_executor_stops_after_external_cancel`: passed.
- `cargo test -p agent-diva-agent --test compaction_real_test --no-run`: passed.
- `cargo fmt --all -- --check`: passed during backlog verification.

## Full gates

- `just fmt-check`: passed.
- `just check`: passed.
- `just test`: failed in `agent-diva-core --lib`.
- A direct `cargo test -p agent-diva-core --lib` rerun passed 612/612.
- A second `cargo test --all` reproduced two intermittent supervised-executor failures:
  - `test_executor_fails_when_no_handler`: observed `Running`, expected `Failed`.
  - `test_executor_stops_after_run_marked_lost`: observed zero handler cancellations, expected one.
- Isolated `--all-features` reruns of both tests passed. The unresolved load-sensitive race is retained in `TODOLIST.md`; it was not expanded into this GUI lifecycle fix.
- `pnpm test` in `agent-diva-gui`: passed, 51 files / 420 tests.
- `pnpm build` in `agent-diva-gui`: passed; Vite retained its existing large-chunk advisory.
- Embedded gateway smoke: the real HTTP listener reached `/api/health` and completed bounded shutdown in `embedded_gateway_serves_health_endpoint`.

## Result

The GUI lifecycle change and all GUI gates pass. The workspace-wide release gate remains red only because of the separately recorded supervised-executor race; no failure from this delivery remained.
