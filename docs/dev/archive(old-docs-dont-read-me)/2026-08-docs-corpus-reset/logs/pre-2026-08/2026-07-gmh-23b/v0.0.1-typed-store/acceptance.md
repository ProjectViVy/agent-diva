# GMH-23B Acceptance

- [x] Empty and repeated open initialize schema v1 and verify FTS5.
- [x] Canonical records round-trip with deterministic ordering and revision CAS.
- [x] Tenant/workspace/session scope is enforced before search or mutation.
- [x] Tombstones and their targets do not remain searchable.
- [x] Invalid, conflicting, corrupt, oversized, and foreign-workspace input fails closed.
- [x] Concurrent same-revision writers produce exactly one winner.
- [x] Restart, integrity, backup, and restore paths are covered.
- [x] 10,000-record top-8 search P95 is below 200ms.
- [x] No production path, user-visible behavior, Mentle removal, or Garden surface changed.
