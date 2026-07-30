# Verification

- `just e7-recovery-drills`: passed.
- `just e7-vertical-e2e`: passed.

The recovery drill covers legacy projection failure windows, typed governed
idempotency and rollback, migration pre/post-commit restoration, canonical
identity backup/rollback, and prepared-journal recovery through receipt
consumption.

The aggregate `just e7-automated-release-gate` remains to be run after E7
observability and side-effect seam reconciliation.
