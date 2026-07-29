# GMH-22 Acceptance

1. Confirm retrieval relevance cannot bypass trust or sensitivity policy.
2. Confirm scope, expiry, tombstone, supersession, and duplicate filtering run
   before token selection.
3. Confirm ranking and budgeting are deterministic and never truncate records.
4. Confirm failed retrieval returns no prompt or stale cache.
5. Confirm trace and shadow reports contain no Memory body.
6. Confirm existing prefetch and AgentLoop behavior remain unchanged.
