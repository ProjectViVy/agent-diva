# GMH-11 Verification

## Focused verification

- `cargo test -p agent-diva-core governance`
  - Passed: 15 tests.
  - Covers precedence, domain capability matrix, full capability/resource
    Cartesian coverage, L0-L4 authorization scope, safe defaults, fixed JSON,
    and stale/tampered/unknown fail-closed behavior.

## Workspace gates

- `just fmt-check`: passed.
- `just check`: passed for the complete workspace with warnings denied.
- `just test`: passed once for the complete workspace in 319.7 seconds. The
  final repeat after adding time-boundary coverage reached one unrelated,
  load-sensitive QQ reconnect fixture failure:
  `qq_falls_back_to_identify_after_invalid_resume_session` observed opcode 6
  before expected opcode 2. Its immediate isolated exact rerun passed. The
  flaky fixture is recorded in `TODOLIST.md`.

Existing non-failing compiler warnings remain outside this iteration: unused
test variables and the `imap-proto 0.10.2` future-incompatibility notice.

This Core-only contract produces no user-visible or executable runtime behavior,
so CLI/GUI smoke testing is not applicable.
