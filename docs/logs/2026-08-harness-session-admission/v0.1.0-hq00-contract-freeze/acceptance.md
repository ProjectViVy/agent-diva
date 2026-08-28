# Acceptance

HQ-00 is accepted when all of the following are true:

- The three baseline characterization tests pass.
- The architecture document freezes defaults, ownership, FIFO/capacity, Stop/Reset/Delete, timeout, eviction,
  failure, shutdown, stable code, and correlation semantics.
- No production queue behavior or public wire/config contract is introduced.
- `TODOLIST.md` marks HQ-00 complete and leaves HQ-01 through HQ-05 open.
- Workspace gates pass, or any unrelated intermittent failure is reproduced, isolated, and durably tracked.

Reviewers should compare `docs/dev/harness-session-admission/hq00-contract.md` with the HQ-01/HQ-02 design
before allowing runtime implementation to proceed.
