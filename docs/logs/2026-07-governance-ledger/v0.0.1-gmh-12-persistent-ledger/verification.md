# GMH-12 Verification

## Focused gates

- `cargo test -p agent-diva-core governance`
  - Passed: 28 tests.
  - Covers JSON contracts, payload/excerpt exclusion, restart replay,
    append-only triggers, CAS races, allow/deny contention, idempotency, TTL,
    tampering, revocation, terminal transitions, and once consumption.
- `cargo test -p agent-diva-core planning`
  - Passed: 70 tests, including the Plan adapter and unchanged legacy JSON.
- `cargo test -p agent-diva-sandbox approval`
  - Passed: 25 tests, including existing coordinator behavior.
- `cargo test -p agent-diva-sandbox governance_adapter`
  - Passed: 2 adapter tests.

## Workspace gates

- `just fmt-check`: passed.
- `just check`: passed for the complete workspace with warnings denied.
- `just test`: final complete-workspace run passed in 281.0 seconds, including
  the previously load-sensitive QQ invalid-resume fixture.

Existing non-failing warnings remain outside this iteration: unused variables in
unrelated tests and the `imap-proto 0.10.2` future-incompatibility notice.

This iteration does not change a production executable path or user-visible
behavior, so CLI/GUI smoke testing is not applicable. The existing
load-sensitive QQ invalid-resume fixture remains tracked in `TODOLIST.md`.
