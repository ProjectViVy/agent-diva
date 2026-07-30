# GMH-23D Crash Recovery Verification

Focused automated coverage verifies:

- recovery when authority committed but the journal still says `prepared`;
- stable replay after receipt consumption;
- one changelog/audit outcome across retry;
- reconstruction of committed outcome from proposal, changelog, audit, and
  rollback artifacts;
- proposal/request/digest/version/idempotency binding.

Validation results:

- `cargo test -p agent-diva-laputa --test service committed_apply_outcome_is_recovered_without_reapplying`: passed.
- `cargo test -p agent-diva-manager laputa_apply_replays_consumed_result_without_duplicate_execution`: passed.
- `cargo test -p agent-diva-manager prepared_journal_recovers_commit_then_consumes_receipt`: passed.
- `just fmt-check`: passed after applying the reported formatting-only change.
- `just check`: passed.
- `just test`: reached `agent-diva-manager --lib` after the preceding workspace
  suites passed, then failed without retaining the individual failing test in
  the bounded output. The immediate isolated
  `cargo test -p agent-diva-manager --lib` rerun passed all 70 tests. This is
  the existing load-sensitive Manager-suite backlog item in `TODOLIST.md`.

Real desktop acceptance is deliberately not represented as automated
verification.
