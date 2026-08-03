# GMH-30B2 Acceptance

## Automated acceptance

- [x] Sandbox, Plan, and Memory production paths receive one coordinator.
- [x] Plan and Memory decisions are version-CAS and digest bound.
- [x] One-time receipts are consumed before executable side effects.
- [x] Restart recovery never invents or replays missing payloads.
- [x] Dangling Allowed work is revoked and resubmitted as fresh Pending work.
- [x] Prepared Consumed work completes idempotently.
- [x] Revoked, Expired, stale, duplicate, and tampered requests fail closed.
- [x] Governance ledger events contain no Plan markdown or Memory patch body.
- [x] Focused suites and all three workspace gates pass.

## Human acceptance

No human action is required at this checkpoint. The operator will perform only
the final M3 manual smoke matrix after GMH-31 through GMH-33 implementation is
complete.
