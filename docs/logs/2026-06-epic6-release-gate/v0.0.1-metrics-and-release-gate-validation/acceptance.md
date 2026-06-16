# Story 6.5 Acceptance

1. Trigger or test an approved Laputa apply path and verify Laputa service metrics expose incremented write counters.
2. Trigger a rollback-eligible governance record and verify rollback validation remains green in the release gate.
3. Trigger a manual AutoDream run path and verify AutoDream service metrics expose run/failure counters without introducing scheduler ownership.
4. Run `just epic6-release-gate` from the workspace root.
5. Confirm the command passes and includes direct-write guard, governance proof loop, Mentle governance boundaries, manual AutoDream stability, and GUI review-surface availability.
