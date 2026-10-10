# Developer acceptance boundary

Current correction body/revision is readable, stale CAS leaves it unchanged, and tombstoned original record remains absent/unreadable after restart. Original reflection/update/delete receipts retain original operation/target/revision. Fresh human create has a distinct ID, not old-record resurrection.

Actual accepted-source replay is stable, changed-event content fails closed, and no extra model requests arise from replay or restart recovery. No user profile or private SQL mutation was used; cursor rewind uses public task-owned Snapshot CAS.

Formal prerequisites and all sample thresholds remain unchanged. ACTMEM, ordinary Agent recall, six-cut crash recovery, degradation/isolation and Windows/live are open. Continue locally executable S03–S11.
