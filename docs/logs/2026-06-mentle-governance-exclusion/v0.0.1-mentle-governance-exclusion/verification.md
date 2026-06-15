# Story 5.2 Mentle Governance Exclusion Verification

## Passed

- `cargo test -p agent-diva-agent mentle`
  - Result: passed, 11 tests.
- `cargo test -p agent-diva-autodream --test outputs`
  - Result: passed, 5 tests.
- `cargo test -p agent-diva-autodream --test reports`
  - Result: passed, 5 tests.
- `cargo test -p agent-diva-autodream --test mentle_governance`
  - Result: passed, 3 tests.
- `cargo test -p agent-diva-laputa --lib`
  - Result: passed, 4 tests.
- `cargo test -p agent-diva-laputa --test mentle_governance`
  - Result: passed, 2 tests.
- `cargo check -p agent-diva-laputa`
  - Result: passed.
- `cargo check -p agent-diva-manager`
  - Result: passed.

## Blocked

- `cargo test -p agent-diva-autodream`
  - Result: failed in unrelated current test `inputs::tests::collector_marks_compaction_capsules_as_secondary_evidence`.
  - Failure: collected compaction excerpt does not contain `secondary evidence only`.
- `cargo test -p agent-diva-laputa`
  - Result: failed to compile unrelated current tests.
  - Failure: tests reference absent API items `LaputaMigrationTestFailure::AfterSectionCommitBeforeState` and `LaputaService::apply_proposal_with_options`.

## Follow-Up

- Blocking validation issues are recorded in root `TODOLIST.md`.
- Story is intentionally not marked `review` until required full validation passes.
